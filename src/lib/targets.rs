/**
 * This is intended to "lazy loop" vectors of strings where syntax like
 * ranges "-" or cidr "/24" might be used to include more values. In these
 * cases, rather than computing the full list and storing in memory, we
 * use a lazy loop to process those extra value "lazily" to save memory.
 */
pub trait LazyLooper<T> {
    fn lazy_loop<F: FnMut(T)>(self, cb: F);
}

pub mod ips;
pub mod ports;
