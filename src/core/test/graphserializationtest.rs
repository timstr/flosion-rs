use crate::{
    core::{
        graphobject::{object_to_sound_processor, ObjectInitialization, WithObjectType},
        graphserialization::{deserialize_sound_graph, serialize_sound_graph},
        object_factory::ObjectFactory,
        serialization::Archive,
        soundgraph::SoundGraph,
    },
    objects::{audioclip::AudioClip, dac::Dac},
};

#[test]
fn test_empty_graph() {
    let g = SoundGraph::new();
    assert_eq!(g.graph_objects().len(), 0);

    let a = Archive::serialize_with(|mut s| serialize_sound_graph(&g, None, &mut s));

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let object_factory = ObjectFactory::new_empty();
    let new_objects = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 0);

    assert_eq!(g2.graph_objects().len(), 0);
}

#[test]
fn test_just_dac() {
    let mut g = SoundGraph::new();
    g.add_sound_processor::<Dac>(ObjectInitialization::Default);
    assert_eq!(g.graph_objects().len(), 1);

    let a = Archive::serialize_with(|mut s| serialize_sound_graph(&g, None, &mut s));

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let mut object_factory = ObjectFactory::new_empty();
    object_factory.register_sound_processor::<Dac>();
    let new_objects = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 1);
    let objs = g2.graph_objects();
    assert_eq!(objs.len(), 1);
    assert_eq!(objs[0].get_type().name(), Dac::TYPE.name());
}

#[test]
fn test_audioclip_to_dac() {
    let mut g = SoundGraph::new();
    let dac = g.add_sound_processor::<Dac>(ObjectInitialization::Default);
    let ac = g.add_sound_processor::<AudioClip>(ObjectInitialization::Default);
    g.connect_sound_input(dac.instance().input.id(), ac.id())
        .unwrap();
    assert_eq!(g.graph_objects().len(), 2);

    let a = Archive::serialize_with(|mut s| serialize_sound_graph(&g, None, &mut s));

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let mut object_factory = ObjectFactory::new_empty();
    object_factory.register_sound_processor::<Dac>();
    object_factory.register_sound_processor::<AudioClip>();
    let new_objects = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 2);
    let objs = g2.graph_objects();
    assert_eq!(objs.len(), 2);

    let mut new_dac = None;
    let mut new_ac = None;
    for o in objs {
        if let Some(x) = object_to_sound_processor::<Dac>(&*o) {
            new_dac = Some(x);
        }
        if let Some(x) = object_to_sound_processor::<AudioClip>(&*o) {
            new_ac = Some(x);
        }
    }
    assert!(new_dac.is_some());
    assert!(new_ac.is_some());

    let new_dac = new_dac.unwrap();
    let new_ac = new_ac.unwrap();

    assert_eq!(
        g2.sound_input_target(new_dac.instance().input.id())
            .unwrap(),
        Some(new_ac.id())
    );
}
