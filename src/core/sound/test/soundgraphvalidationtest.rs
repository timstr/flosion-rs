use crate::core::sound::{
    soundgraphtopology::SoundGraphTopology, soundgraphvalidation::find_error,
};

#[test]
fn find_error_empty_graph() {
    let desc = SoundGraphTopology::new();
    let e = find_error(&desc);
    assert!(e.is_none());
}

// TODO: fix these tests
// #[test]
// fn find_error_one_proc() {
//     let mut topo = SoundGraphTopology::new();
//     let time_nsid = SoundNumberSourceId::new(1);
//     topo.make_sound_edit(SoundEdit::AddSoundProcessor(SoundProcessorData::new(
//         Arc::new(StaticSoundProcessorWithId::new(
//             TestStaticSoundProcessor::new(),
//             SoundProcessorId::new(1),
//             time_nsid,
//         )),
//     )));
//     let e = find_error(&topo);
//     assert!(e.is_none());
// }

// #[test]
// fn find_error_one_proc_cycle() {
//     let mut desc = SoundGraphDescription::new_empty();
//     desc.add_sound_processor(SoundProcessorDescription {
//         id: SoundProcessorId(1),
//         is_static: true,
//         sound_inputs: vec![SoundInputId(1)],
//         number_sources: vec![],
//         number_inputs: vec![],
//     });
//     desc.add_sound_input(SoundInputDescription {
//         id: SoundInputId(1),
//         options: InputOptions::Synchronous,
//         num_keys: 1,
//         target: Some(SoundProcessorId(1)),
//         owner: SoundProcessorId(1),
//         number_sources: vec![],
//     });
//     let e = desc.find_error();
//     let cycle = match e {
//         Some(SoundGraphError::Sound(SoundConnectionError::CircularDependency { cycle })) => cycle,
//         _ => panic!(),
//     };
//     assert!(cycle.contains_processor(SoundProcessorId(1)));
//     assert!(cycle.contains_input(SoundInputId(1)));
//     assert_eq!(cycle.connections.len(), 1);
// }

// #[test]
// fn find_error_two_procs_disconnected() {
//     let mut desc = SoundGraphDescription::new_empty();
//     desc.add_sound_processor(SoundProcessorDescription {
//         id: SoundProcessorId(1),
//         is_static: true,
//         sound_inputs: vec![SoundInputId(1)],
//         number_sources: vec![],
//         number_inputs: vec![],
//     });
//     desc.add_sound_input(SoundInputDescription {
//         id: SoundInputId(1),
//         options: InputOptions::Synchronous,
//         num_keys: 1,
//         target: None,
//         owner: SoundProcessorId(1),
//         number_sources: vec![],
//     });
//     desc.add_sound_processor(SoundProcessorDescription {
//         id: SoundProcessorId(2),
//         is_static: true,
//         sound_inputs: vec![],
//         number_sources: vec![],
//         number_inputs: vec![],
//     });
//     let e = desc.find_error();
//     assert!(e.is_none());
// }

// #[test]
// fn find_error_two_procs_connected() {
//     let mut desc = SoundGraphDescription::new_empty();
//     desc.add_sound_processor(SoundProcessorDescription {
//         id: SoundProcessorId(1),
//         is_static: true,
//         sound_inputs: vec![SoundInputId(1)],
//         number_sources: vec![],
//         number_inputs: vec![],
//     });
//     desc.add_sound_input(SoundInputDescription {
//         id: SoundInputId(1),
//         options: InputOptions::Synchronous,
//         num_keys: 1,
//         target: Some(SoundProcessorId(2)),
//         owner: SoundProcessorId(1),
//         number_sources: vec![],
//     });
//     desc.add_sound_processor(SoundProcessorDescription {
//         id: SoundProcessorId(2),
//         is_static: true,
//         sound_inputs: vec![],
//         number_sources: vec![],
//         number_inputs: vec![],
//     });
//     let e = desc.find_error();
//     assert!(e.is_none());
// }
