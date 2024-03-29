use crate::{
    core::{
        graphobject::{GraphObjectHandle, ObjectInitialization, WithObjectType},
        graphserialization::{deserialize_sound_graph, serialize_sound_graph},
        object_factory::ObjectFactory,
        serialization::Archive,
        soundgraph::SoundGraph,
    },
    objects::{
        audioclip::AudioClip, dac::Dac, functions::SineWave, keyboard::Keyboard,
        wavegenerator::WaveGenerator,
    },
};

#[test]
fn test_empty_graph() {
    let g = SoundGraph::new();
    assert_eq!(g.topology().graph_objects().count(), 0);

    let a = Archive::serialize_with(|mut s| {
        serialize_sound_graph(&g, None, &mut s);
    });

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let object_factory = ObjectFactory::new_empty();
    let (new_objects, _idmap) = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 0);

    assert_eq!(g2.topology().graph_objects().count(), 0);
}

#[test]
fn test_just_dac() {
    let mut g = SoundGraph::new();
    g.add_static_sound_processor::<Dac>(ObjectInitialization::Default)
        .unwrap();
    assert_eq!(g.topology().graph_objects().count(), 1);

    let a = Archive::serialize_with(|mut s| {
        serialize_sound_graph(&g, None, &mut s);
    });

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let mut object_factory = ObjectFactory::new_empty();
    object_factory.register_static_sound_processor::<Dac>();
    let (new_objects, _idmap) = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 1);
    let objs: Vec<_> = g2.topology().graph_objects().collect();
    assert_eq!(objs.len(), 1);
    assert_eq!(objs[0].get_type().name(), Dac::TYPE.name());
}

#[test]
fn test_audioclip_to_dac() {
    let mut g = SoundGraph::new();
    let dac = g
        .add_static_sound_processor::<Dac>(ObjectInitialization::Default)
        .unwrap();
    let ac = g
        .add_dynamic_sound_processor::<AudioClip>(ObjectInitialization::Default)
        .unwrap();
    g.connect_sound_input(dac.input.id(), ac.id()).unwrap();
    assert_eq!(g.topology().graph_objects().count(), 2);

    let a = Archive::serialize_with(|mut s| {
        serialize_sound_graph(&g, None, &mut s);
    });

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let mut object_factory = ObjectFactory::new_empty();
    object_factory.register_static_sound_processor::<Dac>();
    object_factory.register_dynamic_sound_processor::<AudioClip>();
    let (new_objects, _idmap) = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 2);
    let objs: Vec<_> = g2.topology().graph_objects().collect();
    assert_eq!(objs.len(), 2);

    let mut new_dac = None;
    let mut new_ac = None;
    for o in objs {
        if let Some(x) = o.clone().into_static_sound_processor::<Dac>() {
            assert!(new_dac.is_none());
            new_dac = Some(x);
        }
        if let Some(x) = o.into_dynamic_sound_processor::<AudioClip>() {
            assert!(new_ac.is_none());
            new_ac = Some(x);
        }
    }
    assert!(new_dac.is_some());
    assert!(new_ac.is_some());

    let new_dac = new_dac.unwrap();
    let new_ac = new_ac.unwrap();

    assert_eq!(
        g2.topology()
            .sound_input(new_dac.input.id())
            .unwrap()
            .target(),
        Some(new_ac.id())
    );
}

#[test]
fn test_wavegen_keyboard_dac() {
    let mut g = SoundGraph::new();
    let dac = g
        .add_static_sound_processor::<Dac>(ObjectInitialization::Default)
        .unwrap();
    let kbd = g
        .add_static_sound_processor::<Keyboard>(ObjectInitialization::Default)
        .unwrap();
    let wav = g
        .add_dynamic_sound_processor::<WaveGenerator>(ObjectInitialization::Default)
        .unwrap();
    let sin = g
        .add_pure_number_source::<SineWave>(ObjectInitialization::Default)
        .unwrap();
    g.connect_sound_input(dac.input.id(), kbd.id()).unwrap();
    g.connect_sound_input(kbd.input.id(), wav.id()).unwrap();
    g.connect_number_input(wav.frequency.id(), kbd.key_frequency.id())
        .unwrap();
    g.connect_number_input(sin.input.id(), wav.phase.id())
        .unwrap();
    g.connect_number_input(wav.amplitude.id(), sin.id())
        .unwrap();
    assert_eq!(g.topology().graph_objects().count(), 4);

    let a = Archive::serialize_with(|mut s| {
        serialize_sound_graph(&g, None, &mut s);
    });

    let mut g2 = SoundGraph::new();

    let mut d = a.deserialize().unwrap();
    let mut object_factory = ObjectFactory::new_empty();
    object_factory.register_static_sound_processor::<Dac>();
    object_factory.register_static_sound_processor::<Keyboard>();
    object_factory.register_dynamic_sound_processor::<WaveGenerator>();
    object_factory.register_number_source::<SineWave>();
    let (new_objects, _idmap) = deserialize_sound_graph(&mut g2, &mut d, &object_factory).unwrap();

    assert_eq!(new_objects.len(), 4);
    let objs: Vec<GraphObjectHandle> = g2.topology().graph_objects().collect();
    assert_eq!(objs.len(), 4);

    let mut new_dac = None;
    let mut new_kbd = None;
    let mut new_wav = None;
    let mut new_sin = None;
    for o in objs {
        if let Some(x) = o.clone().into_static_sound_processor::<Dac>() {
            assert!(new_dac.is_none());
            new_dac = Some(x);
        }
        if let Some(x) = o.clone().into_static_sound_processor::<Keyboard>() {
            assert!(new_kbd.is_none());
            new_kbd = Some(x);
        }
        if let Some(x) = o.clone().into_dynamic_sound_processor::<WaveGenerator>() {
            assert!(new_wav.is_none());
            new_wav = Some(x);
        }
        if let Some(x) = o.into_pure_number_source::<SineWave>() {
            assert!(new_sin.is_none());
            new_sin = Some(x);
        }
    }
    assert!(new_dac.is_some());
    assert!(new_kbd.is_some());
    assert!(new_wav.is_some());
    assert!(new_sin.is_some());

    let new_dac = new_dac.unwrap();
    let new_kbd = new_kbd.unwrap();
    let new_wav = new_wav.unwrap();
    let new_sin = new_sin.unwrap();

    assert_eq!(
        g2.topology()
            .sound_input(new_dac.input.id())
            .unwrap()
            .target(),
        Some(new_kbd.id())
    );
    assert_eq!(
        g2.topology()
            .sound_input(new_kbd.input.id())
            .unwrap()
            .target(),
        Some(new_wav.id())
    );
    assert_eq!(
        g2.topology()
            .number_input(new_wav.frequency.id())
            .unwrap()
            .target(),
        Some(new_kbd.key_frequency.id())
    );
    assert_eq!(
        g2.topology()
            .number_input(new_sin.input.id())
            .unwrap()
            .target(),
        Some(new_wav.phase.id())
    );
    assert_eq!(
        g2.topology()
            .number_input(new_wav.amplitude.id())
            .unwrap()
            .target(),
        Some(new_sin.id())
    );
}
