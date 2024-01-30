extern crate proc_macro;

use quote::quote;
use rand::Rng;
use syn::{parse_macro_input, token::Extern, Abi, Ident, ItemFn, LitStr};

use crate::proc_macro::TokenStream;
use proc_macro2::Span;

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
