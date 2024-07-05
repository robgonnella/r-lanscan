pub trait LazyLooper<T> {
    fn lazy_loop(self, cb: impl Fn(T));
}

pub mod ips;
pub mod ports;
