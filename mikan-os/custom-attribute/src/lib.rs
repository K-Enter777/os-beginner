extern crate proc_macro;

use proc_macro::TokenTree;
use quote::quote;
use rand::Rng;
use syn::{
    parse_macro_input, spanned::Spanned, token::Extern, Abi, BinOp, Expr, Ident, ItemFn, Lit,
    LitStr, UnOp,
};

use crate::proc_macro::TokenStream;
use proc_macro2::Span;

/// カーネル用のスタックを設定する。
///
/// スタックを設定するアセンブリを、この属性がついている関数名で定義し、そこでスタックを設定する。
/// その後、この属性が付いている関数を呼び出す。
/// このとき、呼び出し規則は System-V ABI が使われる。
///
/// 引数は以下のようにして指定する。
/// * `stack_point` - スタック
/// * `stack_size` - スタックのサイズ
#[proc_macro_attribute]
pub fn kernel_entry(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 引数の処理
    // コンマで区切られた式を集める
    let mut exprs = vec![];
    // 1つの式を構成する要素を集める
    let mut expr = vec![];

    for tree in attr.clone() {
        match tree {
            // コンマ区切りで1つの単位として処理していく
            TokenTree::Punct(punc) => {
                if punc.as_char() == ',' {
                    exprs.push(expr.clone());
                    expr.clear();
                } else {
                    expr.push(TokenTree::Punct(punc));
                }
            }
            _ => expr.push(tree),
        }
    }

    // 終わったときに集めていた式が空でなければ追加する
    if !expr.is_empty() {
        exprs.push(expr);
    }

    // 引数は2つ必要
    if exprs.len() != 2 {
        return syn::Error::new(
            attr.into_iter().next().unwrap().span().into(),
            "引数は2つ必要です。",
        )
        .to_compile_error()
        .into();
    }

    // `TokenTree` の集合を `TokenStream` に変換
    let mut transformed_exprs = vec![];
    for expr in exprs {
        let ts = TokenStream::from_iter(expr);
        transformed_exprs.push(ts);
    }

    // 2つ目の引数が stack_size
    let stack_size = transformed_exprs.pop().unwrap();
    let stack_size = parse_macro_input!(stack_size as Expr);
    let stack_size_span = stack_size.span();
    let stack_size = match into_int(stack_size) {
        Ok(size) => size,
        Err(e) => return e,
    };

    if stack_size <= 0 {
        return syn::Error::new(
            stack_size_span,
            "スタックサイズは正の整数である必要があります。",
        )
        .to_compile_error()
        .into();
    }

    // スタック用変数の名前を得る
    let stack_name = transformed_exprs.pop().unwrap();
    let stack_name = parse_macro_input!(stack_name as Ident);
    let stack_name = quote!(#stack_name);

    // 関数部分の処理
    let mut ast = parse_macro_input!(item as ItemFn);

    let old_entry_name = ast.sig.ident.clone();
    // 元々の関数の新しい名前は、元の名前に impl, 乱数を加えたもの
    // 他の関数名と被らないように乱数を加えている
    let new_entry_name = format!(
        "{}_impl_{}",
        old_entry_name,
        rand::thread_rng().gen::<usize>()
    );

    // 元の関数の名前を新しい名前に変更
    ast.sig.ident = Ident::new(new_entry_name.as_str(), Span::call_site());
    // ブートローダから呼び出せるように、ABI は System-V のもので統一する
    ast.sig.abi = Some(Abi {
        extern_token: Extern {
            span: Span::call_site(),
        },
        name: Some(LitStr::new("sysv64", Span::call_site())),
    });

    // 元の関数の名前が変わらないように、マングリングを抑制する
    let impler = quote! {
        #[no_mangle]
        #ast
    };
    let impler: TokenStream = impler.into();

    // 呼び出し元でスタックの設定を行う
    // quote! 内では format が使えないため、外で文字列にしておく
    let asm = format!(
        r###"
.global {0}
{0}:
    lea rsp, {1} + {2}
    call {3}
.fin:
    hlt
    jmp .fin
"###,
        old_entry_name, stack_name, stack_size, new_entry_name
    );

    let caller = quote! {
        extern "C" {
            fn #old_entry_name();
        }

        core::arch::global_asm! { #asm }
    };
    let mut caller: TokenStream = caller.into();

    // 呼び出し部分と元の関数を1つにする
    caller.extend(impler);

    caller
}

/// ただの関数を x86_64 用の割り込み関数として呼び出せるようにする。
/// そのとき引数として [&InterruptFrame][&kernel::interrupt::InterruptFrame] を受け取れる。
#[proc_macro_attribute]
pub fn interrupt(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as ItemFn);

    let old_ident = ast.sig.ident.clone();
    // 新しい関数名は "impl" と乱数を元の名前の後ろにつける
    let new_fn_name = format!("{}_impl_{}", old_ident, rand::thread_rng().gen::<usize>());

    // 元々の関数の名前を新しい名前に置き換える
    ast.sig.ident = Ident::new(new_fn_name.as_str(), Span::call_site());
    // 呼び出し部分はアセンブリなので、いつでも同じように呼び出せるように、
    // 呼び出し規則を System-V のもので統一する
    ast.sig.abi = Some(Abi {
        extern_token: Extern {
            span: Span::call_site(),
        },
        name: Some(LitStr::new("sysv64", Span::call_site())),
    });

    // 新しく決めた関数名が mangling されないようにする
    let impler = quote! {
        #[no_mangle]
        #ast
    };
    let impler: TokenStream = impler.into();

    // これが呼び出しを行う関数の本体（アセンブリ）
    let asm = format!(
        r###"
.global {0}
{0}:
    push rbp
    mov rbp, rsp
    push rax
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    cld
    lea rdi, [rbp + 0x08]
    call {1}
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11
    pop rax
    mov rsp, rbp
    pop rbp
    iretq
"###,
        old_ident, new_fn_name,
    );

    // 呼び出し部分を元の関数名として宣言、定義する
    // こうすることで、コード上の見た目として同じ関数名で呼び出せる
    let caller = quote! {
        extern "C" {
            fn #old_ident();
        }

        core::arch::global_asm! { #asm }
    };
    let mut caller: TokenStream = caller.into();

    // 呼び出し部分の後ろに元々の関数（改名済）をくっつける
    caller.extend(impler);

    caller
}

/// 式を変換して isize にする。
///
/// 変換できる式は四則演算とビット演算のみ。
/// そうでないものが含まれているなどして失敗した場合は、[Error][syn::Error] から変換された、
/// コンパイルエラーを表す [TokenStream] が返される。
fn into_int(expr: Expr) -> Result<isize, TokenStream> {
    match expr {
        Expr::Lit(lit) => {
            if let Lit::Int(size) = lit.lit {
                match size.base10_parse::<isize>() {
                    Ok(size) => Ok(size),
                    Err(e) => return Err(e.to_compile_error().into()),
                }
            } else {
                return Err(syn::Error::new(lit.span(), "整数を指定してください。")
                    .to_compile_error()
                    .into());
            }
        }
        Expr::Unary(unary) => {
            let size = into_int(*unary.expr)?;
            match unary.op {
                UnOp::Neg(_) => Ok(-size),
                _ => Err(syn::Error::new(unary.op.span(), "不正な演算子です。")
                    .to_compile_error()
                    .into()),
            }
        }
        Expr::Binary(binary) => {
            let left = into_int(*binary.left)?;
            let right = into_int(*binary.right)?;
            match binary.op {
                BinOp::Add(_) => Ok(left + right),
                BinOp::BitAnd(_) => Ok(left & right),
                BinOp::BitOr(_) => Ok(left | right),
                BinOp::BitXor(_) => Ok(left ^ right),
                BinOp::Div(_) => Ok(left / right),
                BinOp::Mul(_) => Ok(left * right),
                BinOp::Rem(_) => Ok(left % right),
                BinOp::Shl(_) => Ok(left << right),
                BinOp::Shr(_) => Ok(left >> right),
                BinOp::Sub(_) => Ok(left - right),
                _ => Err(syn::Error::new(binary.op.span(), "不正な演算子です。")
                    .into_compile_error()
                    .into()),
            }
        }
        Expr::Paren(paren) => into_int(*paren.expr),
        _ => {
            return Err(syn::Error::new(expr.span(), "整数を指定してください。")
                .to_compile_error()
                .into())
        }
    }
}
