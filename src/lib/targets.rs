pub trait LazyLooper<T> {
    fn lazy_loop<F: FnMut(T)>(self, cb: F);
}

pub mod ips;
pub mod ports;
