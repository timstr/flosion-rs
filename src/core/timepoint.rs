use super::uniqueid::{IdGenerator, UniqueId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TimePointId(usize);

impl UniqueId for TimePointId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        TimePointId(self.0 + 1)
    }
}

// TODO: ??? remove or move elsewhere
#[derive(Clone, Copy)]
pub struct Samples(usize);

pub struct TimePoint {}

// A description of how to create time points,
// possibly dependent on other time points
pub enum TimePointRecipe {
    Fixed(Samples),
    After(TimePointId, Samples),
    // Between(TimePointId, TimePointId, f32)
    // Repeat(...)
    // Subdivide(...)
    // Unplanned(...)
    // etc...
}

impl TimePointRecipe {
    // Can all time points in the recipe be determined
    // before any audio processing happens?
    pub fn can_be_scheduled() -> bool {
        todo!();
    }

    pub fn make_generator(&self) -> TimePointGenerator {
        match self {
            TimePointRecipe::Fixed(s) => TimePointGenerator::Fixed(*s),
            TimePointRecipe::After(tp, s) => TimePointGenerator::After(*tp, *s),
        }
    }

    fn depends_on(&self, id: TimePointId) -> bool {
        match self {
            TimePointRecipe::Fixed(_) => false,
            TimePointRecipe::After(tp, _) => *tp == id,
        }
    }
}

// An object for producing time points on demand,
// intended for audio processing but also usable
// in the GUI
enum TimePointGenerator {
    Fixed(Samples),
    After(TimePointId, Samples),
    // TODO: others
}

impl TimePointGenerator {
    fn start_over(&mut self) {}
}

// A set of time point recipes
pub struct Timeline {
    // must be sorted in topological order
    recipes: Vec<(TimePointId, TimePointRecipe)>,

    id_generator: IdGenerator<TimePointId>,
}

impl Timeline {
    pub fn add_recipe(&mut self, recipe: TimePointRecipe) -> TimePointId {
        let id = self.id_generator.next_id();

        todo!()
    }
}
