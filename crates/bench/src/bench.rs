use light_contracts::BenchDescriptor;

/// A single measurable workload.
///
/// `setup` runs once before warmup; `iter` is the timed body. Implementations
/// should `std::hint::black_box` any results they compute.
pub trait Bench: Send {
    fn descriptor(&self) -> BenchDescriptor;

    fn setup(&mut self) {}

    fn iter(&mut self);
}
