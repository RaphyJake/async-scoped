//! There you can find traits that are necessary for implementing executor.
//! They are mostly unsafe, as we can't ensure the executor allows intended manipulations
//! without causing UB.
use futures::Future;

pub unsafe trait SpawnerLocal<T> {
    type FutureOutput;
    type SpawnHandle: Future<Output = Self::FutureOutput>;
    fn spawn_local<F: Future<Output = T> + 'static>(&self, f: F) -> Self::SpawnHandle;
}

pub unsafe trait Spawner<T>: SpawnerLocal<T> {
    fn spawn<F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> <Self as SpawnerLocal<T>>::SpawnHandle;
}

pub unsafe trait FuncSpawner<T> {
    type FutureOutput;
    type SpawnHandle: Future<Output = Self::FutureOutput> + Send;
    fn spawn_func<F: FnOnce() -> T + Send + 'static>(&self, f: F) -> Self::SpawnHandle;
}

pub unsafe trait Blocker {
    fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T;
}

#[cfg(feature = "use-async-std")]
pub mod use_async_std {
    use super::*;
    use async_std::task::{block_on, spawn, spawn_blocking, JoinHandle};

    #[derive(Default)]
    pub struct AsyncStd;

    unsafe impl<T: 'static> SpawnerLocal<T> for AsyncStd {
        type FutureOutput = T;
        type SpawnHandle = JoinHandle<T>;

        fn spawn_local<F: Future<Output = T> + 'static>(&self, f: F) -> Self::SpawnHandle {
            unimplemented!();
        }
    }

    unsafe impl<T: Send + 'static> Spawner<T> for AsyncStd {
        fn spawn<F: Future<Output = T> + Send + 'static>(&self, f: F) -> Self::SpawnHandle {
            spawn(f)
        }
    }
    unsafe impl<T: Send + 'static> FuncSpawner<T> for AsyncStd {
        type FutureOutput = T;
        type SpawnHandle = JoinHandle<T>;

        fn spawn_func<F: FnOnce() -> T + Send + 'static>(&self, f: F) -> Self::SpawnHandle {
            spawn_blocking(f)
        }
    }
    unsafe impl Blocker for AsyncStd {
        fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
            block_on(f)
        }
    }
}

#[cfg(feature = "use-tokio")]
pub mod use_tokio {
    use super::*;
    use tokio::{runtime::Handle, task as tokio_task};

    #[derive(Default)]
    pub struct Tokio;

    unsafe impl<T: 'static> SpawnerLocal<T> for Tokio {
        type FutureOutput = Result<T, tokio_task::JoinError>;
        type SpawnHandle = tokio_task::JoinHandle<T>;

        fn spawn_local<F: Future<Output = T> + 'static>(&self, f: F) -> Self::SpawnHandle {
            tokio::task::spawn_local(f)
        }
    }

    unsafe impl<T: Send + 'static> Spawner<T> for Tokio {
        fn spawn<F: Future<Output = T> + Send + 'static>(&self, f: F) -> Self::SpawnHandle {
            tokio_task::spawn(f)
        }
    }

    unsafe impl<T: Send + 'static> FuncSpawner<T> for Tokio {
        type FutureOutput = Result<T, tokio_task::JoinError>;
        type SpawnHandle = tokio_task::JoinHandle<T>;

        fn spawn_func<F: FnOnce() -> T + Send + 'static>(&self, f: F) -> Self::SpawnHandle {
            tokio_task::spawn_blocking(f)
        }
    }

    unsafe impl Blocker for Tokio {
        fn block_on<T, F: Future<Output = T>>(&self, f: F) -> T {
            let rt = Handle::current();

            match rt.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::CurrentThread => {
                    rt.block_on(f) // use current runtime directly
                }
                tokio::runtime::RuntimeFlavor::MultiThread => tokio_task::block_in_place(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(f)
                }),
                _ => todo!(),
            }
        }
    }
}
