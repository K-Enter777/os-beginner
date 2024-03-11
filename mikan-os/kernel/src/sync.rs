use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
};

/// [RwLock] 等が借用されていないときの `count` の値。
const UNUSED: isize = 0;
/// [RwLock] 等が不変参照される最大値。
const BORROW_MAX: isize = isize::MAX / 2;

/// スレッドセーフな内部可変性を持つ構造体。
pub(crate) struct RwLock<T> {
    /// 内部データを持つ。
    data: UnsafeCell<T>,
    /// 参照カウンタ。
    counter: AtomicIsize,
    /// 不変参照を作るときに参照カウンタをインクリメントするが、
    /// このときに可変参照が作られていて、参照カウンタが0になってしまっていると、
    /// 新しく可変参照が作れてしまう。
    /// それを阻止するのに、不変参照作成時に参照カウンタをロックするのに使う。
    locker: AtomicBool,
}

unsafe impl<T: Send> Sync for RwLock<T> {}
unsafe impl<T: Send> Send for RwLock<T> {}

impl<T> RwLock<T> {
    /// [RwLock] のコンストラクタ。
    pub(crate) const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            counter: AtomicIsize::new(0),
            locker: AtomicBool::new(false),
        }
    }

    /// 参照カウンタのためのロックを得る。
    ///
    /// ロックを得るまで無限に待機する。
    fn lock(&self) -> LockKey<'_> {
        while let Err(_) =
            self.locker
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
        {}

        LockKey {
            locker: &self.locker,
        }
    }

    /// 読み取りのための不変参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub(crate) fn read(&self) -> RwRead<'_, T> {
        loop {
            let _locker = self.lock();
            let prev_count = self.counter.fetch_add(1, Ordering::Release);
            // 元々参照がない or 不変参照しかない状況で、
            // 不変参照の数が `BORROW_MAX` 以下のとき、新たな参照を作って返す。
            // それまでは待機する。
            if prev_count >= UNUSED && prev_count <= BORROW_MAX {
                break;
            }
            self.counter.fetch_sub(1, Ordering::Release);
        }

        RwRead {
            data: unsafe { &*self.data.get() },
            counter: &self.counter,
        }
    }

    /// 書き込みのための可変参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub(crate) fn write(&self) -> RwWrite<'_, T> {
        loop {
            let prev_count = self.counter.fetch_sub(1, Ordering::Release);
            // 元々参照が作られていない場合に、新たに可変参照を作って返す。
            // それまでは待機する。
            if prev_count == UNUSED {
                break;
            }
            self.counter.fetch_add(1, Ordering::Release);
        }

        RwWrite {
            data: unsafe { &mut *self.data.get() },
            counter: &self.counter,
        }
    }
}

/// 可変の static 変数として使える、実行時に初期化が可能な [RwLock]。
///
/// ただし、初期化していないアクセスは `panic!` を引き起こすので注意。
pub(crate) struct OnceRwLock<T> {
    /// 内部データを持つ。
    data: UnsafeCell<Option<T>>,
    /// 参照カウンタ。
    counter: AtomicIsize,
    /// 不変参照を作るときに参照カウンタをインクリメントするが、
    /// このときに可変参照が作られていて、参照カウンタが0になってしまっていると、
    /// 新しく可変参照が作れてしまう。
    /// それを阻止するのに、不変参照作成時に参照カウンタをロックするのに使う。
    locker: AtomicBool,
}

unsafe impl<T: Send> Sync for OnceRwLock<T> {}
unsafe impl<T: Send> Send for OnceRwLock<T> {}

impl<T> OnceRwLock<T> {
    /// [OnceMutex] のコンストラクタ。
    pub(crate) const fn new() -> Self {
        Self {
            data: UnsafeCell::new(None),
            counter: AtomicIsize::new(0),
            locker: AtomicBool::new(false),
        }
    }

    /// [OnceMutex] の、初期化も行うコンストラクタ。
    ///
    /// * `value` - 設定する値。
    pub(crate) const fn from_value(value: T) -> Self {
        Self {
            data: UnsafeCell::new(Some(value)),
            counter: AtomicIsize::new(0),
            locker: AtomicBool::new(false),
        }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        let _locker = self.lock();
        unsafe { &*self.data.get() }.is_some()
    }

    /// [OnceMutex] の初期化を行う。
    ///
    /// * `value` - 設定する値。
    ///
    /// # 戻り値
    /// 初期化できたかどうかを返す。
    ///
    /// 既に初期化されている場合は初期化されない。
    pub(crate) fn init(&self, value: T) -> bool {
        let _locker = self.lock();
        while let Err(_) =
            self.counter
                .compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed)
        {}

        let data = self.data.get();
        match unsafe { &*data } {
            Some(_) => {
                self.counter.fetch_add(1, Ordering::Release);
                false
            }
            None => {
                unsafe { *data = Some(value) };
                self.counter.fetch_add(1, Ordering::Release);
                true
            }
        }
    }

    /// 参照カウンタのためのロックを得る。
    ///
    /// ロックを得るまで無限に待機する。
    fn lock(&self) -> LockKey<'_> {
        while let Err(_) =
            self.locker
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
        {}

        LockKey {
            locker: &self.locker,
        }
    }

    /// 読み取りのための不変参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub(crate) fn read(&self) -> RwRead<'_, T> {
        self.read_checked()
            .unwrap_or_else(|| panic!("uninitialized value was accessed."))
    }

    /// 読み取りのための不変参照を取得する。
    ///
    /// [read()][Self::read()] との違いは、初期化が行われていない場合に `panic!` でなく、
    /// `None` が返ること。
    pub(crate) fn read_checked(&self) -> Option<RwRead<'_, T>> {
        loop {
            let _locker = self.lock();
            let prev_count = self.counter.fetch_add(1, Ordering::Release);
            // 元々参照がない or 不変参照しかない状況で、
            // 不変参照の数が `BORROW_MAX` 以下のとき、新たな参照を作って返す。
            // それまでは待機する。
            if prev_count >= UNUSED && prev_count <= BORROW_MAX {
                break;
            }
            self.counter.fetch_sub(1, Ordering::Release);
        }

        match unsafe { (*self.data.get()).as_ref() } {
            Some(data) => Some(RwRead {
                data: data,
                counter: &self.counter,
            }),
            None => None,
        }
    }

    /// 書き込みのための可変参照を取得する。
    ///
    /// 参照が得られるまで無限に待機する。
    pub(crate) fn write(&self) -> RwWrite<'_, T> {
        self.write_checked()
            .unwrap_or_else(|| panic!("uninitialized value has accessd"))
    }

    /// 書き込みのための可変参照を取得する。
    ///
    /// [write()][Self::write()] との違いは、初期化が行われていない場合に `panic!` でなく、
    /// `None` が返ること。
    pub(crate) fn write_checked(&self) -> Option<RwWrite<'_, T>> {
        loop {
            let _locker = self.lock();
            let prev_count = self.counter.fetch_sub(1, Ordering::Release);
            // 元々参照が作られていない場合に、新たに可変参照を作って返す。
            // それまでは待機する。
            if prev_count == UNUSED {
                break;
            }
            self.counter.fetch_add(1, Ordering::Release);
        }

        match unsafe { (*self.data.get()).as_mut() } {
            Some(data) => Some(RwWrite {
                data: data,
                counter: &self.counter,
            }),
            None => None,
        }
    }
}

/// [OnceMutex] の `count` の操作をロック、ロック解除するときに使用される構造体。
struct LockKey<'this> {
    locker: &'this AtomicBool,
}

impl<'this> Drop for LockKey<'this> {
    fn drop(&mut self) {
        self.locker.store(false, Ordering::Release);
    }
}

/// [RwLock] を読み取るときに、そのデータと参照カウンタを管理するために使われる構造体。
pub(crate) struct RwRead<'this, T> {
    data: &'this T,
    counter: &'this AtomicIsize,
}

impl<'this, T> Deref for RwRead<'this, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'this, T> Drop for RwRead<'this, T> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Release);
    }
}

/// [RwLock] に書き込むときに、そのデータと参照カウンタを管理するために使われる構造体。
pub(crate) struct RwWrite<'this, T> {
    data: &'this mut T,
    counter: &'this AtomicIsize,
}

impl<'this, T> Deref for RwWrite<'this, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'this, T> DerefMut for RwWrite<'this, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'this, T> Drop for RwWrite<'this, T> {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::Release);
    }
}
