use std::collections::HashSet;

use crate::core::numbersource::NumberSourceOwner;

use super::{
    graphobject::ObjectId,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    object_factory::ObjectFactory,
    serialization::{Deserializer, Serializer},
    soundgraph::SoundGraph,
    soundgraphdata::{EngineNumberSourceData, EngineSoundProcessorData},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

struct ForwardIdMap<T: UniqueId> {
    ids: Vec<T>,
}

impl<T: UniqueId> ForwardIdMap<T> {
    fn new() -> ForwardIdMap<T> {
        ForwardIdMap { ids: Vec::new() }
    }

    fn add_id(&mut self, id: T) {
        if self.map_id(id).is_some() {
            return;
        }
        self.ids.push(id);
    }

    fn map_id(&self, id: T) -> Option<u16> {
        self.ids.iter().position(|i| *i == id).map(|i| i as u16)
    }

    fn len(&self) -> usize {
        self.ids.len()
    }
}

struct ForwardGraphIdMap {
    sound_processors: ForwardIdMap<SoundProcessorId>,
    sound_inputs: ForwardIdMap<SoundInputId>,
    number_sources: ForwardIdMap<NumberSourceId>,
    number_inputs: ForwardIdMap<NumberInputId>,
}

impl ForwardGraphIdMap {
    fn new() -> ForwardGraphIdMap {
        ForwardGraphIdMap {
            sound_processors: ForwardIdMap::new(),
            sound_inputs: ForwardIdMap::new(),
            number_sources: ForwardIdMap::new(),
            number_inputs: ForwardIdMap::new(),
        }
    }

    fn visit_sound_processor_data(&mut self, proc_data: &EngineSoundProcessorData) {
        self.sound_processors.add_id(proc_data.id());
        for x in proc_data.sound_inputs() {
            self.sound_inputs.add_id(*x);
        }
        for x in proc_data.number_sources() {
            self.number_sources.add_id(*x);
        }
        for x in proc_data.number_inputs() {
            self.number_inputs.add_id(*x);
        }
    }

    fn visit_number_source_data(&mut self, src_data: &EngineNumberSourceData) {
        self.number_sources.add_id(src_data.id());
        for x in src_data.inputs() {
            self.number_inputs.add_id(*x);
        }
    }

    fn serialize(&self, serializer: &mut Serializer) {
        serializer.u16(self.sound_processors.len() as u16);
        serializer.u16(self.sound_inputs.len() as u16);
        serializer.u16(self.number_sources.len() as u16);
        serializer.u16(self.number_inputs.len() as u16);
    }
}

struct ReverseIdMap<T: UniqueId> {
    ids: Vec<Option<T>>,
}

impl<T: UniqueId> ReverseIdMap<T> {
    fn new(len: usize) -> ReverseIdMap<T> {
        ReverseIdMap {
            ids: (0..len).map(|_| None).collect(),
        }
    }

    fn add_id(&mut self, serialization_id: u16, new_id: T) -> Result<(), ()> {
        let i = serialization_id as usize;
        if i >= self.ids.len() {
            return Err(());
        }
        let id = &mut self.ids[i];
        if id.is_some() {
            return Err(());
        }
        *id = Some(new_id);
        Ok(())
    }

    fn map_id(&self, serialization_id: u16) -> T {
        self.ids[serialization_id as usize].unwrap()
    }

    fn is_full(&self) -> bool {
        self.ids.iter().all(|i| i.is_some())
    }
}

struct ReverseGraphIdMap {
    sound_processors: ReverseIdMap<SoundProcessorId>,
    sound_inputs: ReverseIdMap<SoundInputId>,
    number_sources: ReverseIdMap<NumberSourceId>,
    number_inputs: ReverseIdMap<NumberInputId>,
}

impl ReverseGraphIdMap {
    fn deserialize(deserializer: &mut Deserializer) -> Result<ReverseGraphIdMap, ()> {
        let sound_processors = deserializer.u16()? as usize;
        let sound_inputs = deserializer.u16()? as usize;
        let number_sources = deserializer.u16()? as usize;
        let number_inputs = deserializer.u16()? as usize;
        Ok(ReverseGraphIdMap {
            sound_processors: ReverseIdMap::new(sound_processors),
            sound_inputs: ReverseIdMap::new(sound_inputs),
            number_sources: ReverseIdMap::new(number_sources),
            number_inputs: ReverseIdMap::new(number_inputs),
        })
    }

    fn add_sound_processor(
        &mut self,
        serialized_id: u16,
        new_id: SoundProcessorId,
    ) -> Result<(), ()> {
        self.sound_processors.add_id(serialized_id, new_id)
    }

    fn add_sound_input(&mut self, serialized_id: u16, new_id: SoundInputId) -> Result<(), ()> {
        self.sound_inputs.add_id(serialized_id, new_id)
    }

    fn add_number_source(&mut self, serialized_id: u16, new_id: NumberSourceId) -> Result<(), ()> {
        self.number_sources.add_id(serialized_id, new_id)
    }

    fn add_number_input(&mut self, serialized_id: u16, new_id: NumberInputId) -> Result<(), ()> {
        self.number_inputs.add_id(serialized_id, new_id)
    }

    fn is_full(&self) -> bool {
        self.sound_processors.is_full()
            && self.sound_inputs.is_full()
            && self.number_sources.is_full()
            && self.number_inputs.is_full()
    }
}

pub fn serialize_sound_graph(
    graph: &SoundGraph,
    subset: Option<&HashSet<ObjectId>>,
    serializer: &mut Serializer,
) {
    let is_selected = |id: ObjectId| match subset {
        Some(s) => s.get(&id).is_some(),
        None => true,
    };

    let topo = graph.topology();
    let topo = topo.read();

    // 1. visit all objects and note their associated ids (do this first so that
    //    during deserialization, ids can be repopulated while objects are being
    //    deserialized in the second step). Serialize the number of each type of id.
    let mut idmap = ForwardGraphIdMap::new();
    let mut num_sound_processors: usize = 0;
    let mut num_number_sources: usize = 0;
    for pd in topo.sound_processors().values() {
        if is_selected(pd.id().into()) {
            idmap.visit_sound_processor_data(pd);
            num_sound_processors += 1;
        }
    }
    for ns in topo.number_sources().values() {
        if is_selected(ns.id().into()) {
            debug_assert!(ns.owner() == NumberSourceOwner::Nothing);
            idmap.visit_number_source_data(ns);
            num_number_sources += 1;
        }
    }
    idmap.serialize(serializer);

    // 2. visit each selected object and serialize
    //     2a. its own mapped id
    //     2b. the mapped ids of its sound inputs (for sound processors)
    //     2c. the mapped ids of its number sources (for sound processors)
    //     2d. the mapped ids of its number inputs
    //     2e. the type name of the object (for object factory)
    //     2f. the object instance
    //         NOTE that sound processors will be responsible for (de)serializing
    //         multiinput keys
    serializer.u16(num_sound_processors as u16);
    for pd in topo.sound_processors().values() {
        if !is_selected(pd.id().into()) {
            continue;
        }
        let mut s1 = serializer.subarchive();
        // the sound processor's own id
        s1.u16(idmap.sound_processors.map_id(pd.id()).unwrap());
        // the sound input ids
        s1.array_iter_u16(
            pd.sound_inputs()
                .iter()
                .map(|x| idmap.sound_inputs.map_id(*x).unwrap()),
        );
        // the number source ids
        s1.array_iter_u16(
            pd.number_sources()
                .iter()
                .map(|x| idmap.number_sources.map_id(*x).unwrap()),
        );
        // the number input ids
        s1.array_iter_u16(
            pd.number_inputs()
                .iter()
                .map(|x| idmap.number_inputs.map_id(*x).unwrap()),
        );
        let obj = pd.processor_arc().as_graph_object(pd.id());
        // the type name
        s1.string(obj.get_type().name());
        // the instance itself
        let s2 = s1.subarchive();
        obj.serialize(s2);
    }
    // TODO: same for number source objects
    serializer.u16(num_number_sources as u16);
    for ns in topo.number_sources().values() {
        if !is_selected(ns.id().into()) {
            continue;
        }
        debug_assert!(ns.owner() == NumberSourceOwner::Nothing);
        let mut s1 = serializer.subarchive();
        // the number sources' own id
        s1.u16(idmap.number_sources.map_id(ns.id()).unwrap());
        // the number input ids
        s1.array_iter_u16(
            ns.inputs()
                .iter()
                .map(|x| idmap.number_inputs.map_id(*x).unwrap()),
        );
        let obj = ns.instance_arc().as_graph_object(ns.id()).unwrap();
        // the type name
        s1.string(obj.get_type().name());
        // the instance itself
        let s2 = s1.subarchive();
        obj.serialize(s2);
    }

    // 3. serialize all sound/number connections between ids that were visited in step 1
    serializer.array_iter_u16(
        topo.sound_inputs()
            .values()
            .filter_map(|si| {
                let t = match si.target() {
                    Some(t) => t,
                    None => return None,
                };
                let i = idmap.sound_inputs.map_id(si.id());
                let o = idmap.sound_processors.map_id(t);
                if i.is_none() || o.is_none() {
                    return None;
                }
                Some([i.unwrap(), o.unwrap()])
            })
            .flatten(),
    );

    serializer.array_iter_u16(
        topo.number_inputs()
            .values()
            .filter_map(|si| {
                let t = match si.target() {
                    Some(t) => t,
                    None => return None,
                };
                let i = idmap.number_inputs.map_id(si.id());
                let o = idmap.number_sources.map_id(t);
                if i.is_none() || o.is_none() {
                    return None;
                }
                Some([i.unwrap(), o.unwrap()])
            })
            .flatten(),
    );
}

pub fn deserialize_sound_graph(
    dst_graph: &mut SoundGraph,
    deserializer: &mut Deserializer,
    object_factory: &ObjectFactory,
) -> Result<Vec<ObjectId>, ()> {
    // TODO
    // - refer to serialize_sound_graph above
    // - add new objects to dst_graph
    // - return the ids of all newly created objects

    let mut new_objects: Vec<ObjectId> = Vec::new();

    // 1. Deserialize the initial id mapping
    let mut idmap = ReverseGraphIdMap::deserialize(deserializer)?;

    // 2. deserialize each object and
    //     2a. its own mapped id
    //     2b. mapped ids of sound inputs (for sound processors)
    //     2c. mapped ids of number sources (for sound processors)
    //     2d. mapped ids of number inputs
    //     2e. the type name of the object
    //     2f. the instance itself, using type name and factory
    // then map id of new object, and ensure that number of
    // sound/number inputs/sources match and map their new id
    // in order to the serialized ids
    let num_sound_processors = deserializer.u16()? as usize;
    for _ in 0..num_sound_processors {
        let mut s1 = deserializer.subarchive()?;
        let spid = s1.u16()?;
        let siids = s1.array_slice_u16()?;
        let nsids = s1.array_slice_u16()?;
        let niids = s1.array_slice_u16()?;
        let name = s1.string()?;
        let s2 = deserializer.subarchive()?;
        let new_sp = object_factory.create_from_archive(&name, dst_graph, s2);
        new_objects.push(new_sp.get_id());
        let new_spid = match new_sp.get_id() {
            ObjectId::Sound(i) => i,
            ObjectId::Number(_) => return Err(()),
        };
        idmap.add_sound_processor(spid, new_spid)?;
        let topo = dst_graph.topology();
        let topo = topo.read();
        let sp_data = topo.sound_processors().get(&new_spid).unwrap();
        if sp_data.sound_inputs().len() != siids.len() {
            println!(
                "Wrong number of sound inputs deserialized for sound processor \"{}\"",
                name
            );
            return Err(());
        }
        if sp_data.number_sources().len() != nsids.len() {
            println!(
                "Wrong number of number sources deserialized for sound processor \"{}\"",
                name
            );
            return Err(());
        }
        if sp_data.number_inputs().len() != niids.len() {
            println!(
                "Wrong number of number inputs deserialized for sound processor \"{}\"",
                name
            );
            return Err(());
        }
        for (old_id, new_id) in siids.iter().zip(sp_data.sound_inputs()) {
            idmap.add_sound_input(*old_id, *new_id)?;
        }
        for (old_id, new_id) in nsids.iter().zip(sp_data.number_sources()) {
            idmap.add_number_source(*old_id, *new_id)?;
        }
        for (old_id, new_id) in niids.iter().zip(sp_data.number_inputs()) {
            idmap.add_number_input(*old_id, *new_id)?;
        }
    }

    let num_number_sources = deserializer.u16()? as usize;
    for _ in 0..num_number_sources {
        let mut s1 = deserializer.subarchive()?;
        let spid = s1.u16()?;
        let niids = s1.array_slice_u16()?;
        let name = s1.string()?;
        let s2 = deserializer.subarchive()?;
        let new_ns = object_factory.create_from_archive(&name, dst_graph, s2);
        new_objects.push(new_ns.get_id());
        let new_nsid = match new_ns.get_id() {
            ObjectId::Sound(_) => return Err(()),
            ObjectId::Number(i) => i,
        };
        idmap.add_number_source(spid, new_nsid)?;
        let topo = dst_graph.topology();
        let topo = topo.read();
        let ns_data = topo.number_sources().get(&new_nsid).unwrap();
        if ns_data.inputs().len() != niids.len() {
            println!(
                "Wrong number of number inputs deserialized for sound processor \"{}\"",
                name
            );
            return Err(());
        }
        for (old_id, new_id) in niids.iter().zip(ns_data.inputs()) {
            idmap.add_number_input(*old_id, *new_id)?;
        }
    }

    if !idmap.is_full() {
        return Err(());
    }

    // 3. deserialize all sound and number connections by mapping
    // serialized ids to the newly-created object ids from step 2
    if deserializer.peek_length()? % 2 != 0 {
        return Err(());
    }
    let mut sid_iter = deserializer.array_iter_u16()?;
    while let Some(old_siid) = sid_iter.next() {
        let old_spid = sid_iter.next().unwrap();
        let new_siid = idmap.sound_inputs.map_id(old_siid);
        let new_spid = idmap.sound_processors.map_id(old_spid);
        dst_graph.connect_sound_input(new_siid, new_spid).unwrap();
    }

    if deserializer.peek_length()? % 2 != 0 {
        return Err(());
    }
    let mut nid_iter = deserializer.array_iter_u16()?;
    while let Some(old_niid) = nid_iter.next() {
        let old_nsid = nid_iter.next().unwrap();
        let new_niid = idmap.number_inputs.map_id(old_niid);
        let new_nsid = idmap.number_sources.map_id(old_nsid);
        dst_graph.connect_number_input(new_niid, new_nsid).unwrap();
    }

    Ok(new_objects)
}
