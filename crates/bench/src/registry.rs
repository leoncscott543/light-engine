use light_contracts::{BenchDescriptor, BenchId};

use crate::bench::Bench;

type Factory = Box<dyn Fn() -> Box<dyn Bench> + Send + Sync>;

/// Catalog of available benchmarks.
///
/// Benchmarks are stored as factories so each run gets a fresh instance,
/// which keeps runs independent and lets them move onto worker threads.
#[derive(Default)]
pub struct Registry {
    entries: Vec<(BenchDescriptor, Factory)>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// The built-in benchmark set. Intentionally empty until the kernel lands.
    pub fn builtin() -> Self {
        Self::new()
    }

    pub fn register<B, F>(&mut self, factory: F) -> &mut Self
    where
        B: Bench + 'static,
        F: Fn() -> B + Send + Sync + 'static,
    {
        let descriptor = factory().descriptor();
        self.entries.push((
            descriptor,
            Box::new(move || Box::new(factory()) as Box<dyn Bench>),
        ));
        self
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &BenchDescriptor> {
        self.entries.iter().map(|(d, _)| d)
    }

    pub fn instantiate(&self, id: &BenchId) -> Option<Box<dyn Bench>> {
        self.entries
            .iter()
            .find(|(d, _)| &d.id == id)
            .map(|(_, factory)| factory())
    }
}
