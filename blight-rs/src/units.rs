const KB1: u64 = 1024;

pub const fn kilobytes(n: u64) -> u64 {
    n * KB1
}

pub const fn megabytes(n: u64) -> u64 {
    kilobytes(n) * KB1
}

pub const fn gigabytes(n: u64) -> u64 {
    megabytes(n) * KB1
}

pub const fn terabytes(n: u64) -> u64 {
    gigabytes(n) * KB1
}
