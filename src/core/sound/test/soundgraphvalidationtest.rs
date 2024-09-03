use crate::{
    core::sound::{
        sounderror::SoundError,
        soundgraphdata::SoundInputBranchId,
        soundgraphtopology::SoundGraphTopology,
        soundgraphvalidation::find_sound_error,
        soundinput::InputOptions,
        test::testobjects::{TestDynamicSoundProcessor, TestStaticSoundProcessor},
        topologyedits::{
            build_dynamic_sound_processor, build_sound_input, build_static_sound_processor,
            SoundGraphIdGenerators,
        },
    },
    ui_core::arguments::ParsedArguments,
};

#[test]
fn find_error_empty_graph() {
    let topo = SoundGraphTopology::new();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_one_static_proc() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();
    build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_one_dynamic_proc() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();
    build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_self_cycle() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc.id(),
        InputOptions::Synchronous,
        Vec::new(),
    );

    topo.connect_sound_input(input_id, proc.id()).unwrap();

    assert!(match find_sound_error(&topo) {
        Some(SoundError::CircularDependency { cycle: _ }) => true,
        _ => false,
    });
}

#[test]
fn find_error_static_to_dynamic_no_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let static_proc = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let dynamic_proc = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        static_proc.id(),
        InputOptions::Synchronous,
        vec![],
    );

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_dynamic_one_branch() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let static_proc = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let dynamic_proc = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        static_proc.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_dynamic_two_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let static_proc = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let dynamic_proc = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        static_proc.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
    );

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_no_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_static_to_static_one_branch() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_two_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_static_to_dynamic_one_branch_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::NonSynchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_one_branch_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::NonSynchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_no_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_one_branch() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_to_static_two_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_id = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::NonSynchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_no_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc3 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![],
    );

    let input2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_one_branch() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc3 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_cycle() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc3 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input3 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc3.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();
    topo.connect_sound_input(input3, proc1.id()).unwrap();

    assert!(match find_sound_error(&topo) {
        Some(SoundError::CircularDependency { cycle: _ }) => true,
        _ => false,
    });
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_two_branches() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc3 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
    );

    let input2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc3 = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc1.id(),
        InputOptions::NonSynchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_indirect_fork_to_static_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc_root1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_root2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_middle = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_leaf = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_root1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_root1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input_root2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_root2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input_middle = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_middle.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_root1, proc_middle.id())
        .unwrap();
    topo.connect_sound_input(input_root2, proc_middle.id())
        .unwrap();
    topo.connect_sound_input(input_middle, proc_leaf.id())
        .unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc_leaf.id()))
    );
}

#[test]
fn find_error_dynamic_direct_fork_to_static_nonsync() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc_root1 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_root2 = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_leaf = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_root1 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_root1.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input_root2 = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_root2.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_root1, proc_leaf.id())
        .unwrap();
    topo.connect_sound_input(input_root2, proc_leaf.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_split_to_static_two_inputs() {
    let mut topo = SoundGraphTopology::new();
    let mut idgens = SoundGraphIdGenerators::new();

    let proc_root = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_side = build_dynamic_sound_processor::<TestDynamicSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let proc_leaf = build_static_sound_processor::<TestStaticSoundProcessor>(
        &mut topo,
        &mut idgens,
        &ParsedArguments::new_empty(),
    )
    .unwrap();

    let input_side = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_side.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input_leaf_a = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_leaf.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    let input_leaf_b = build_sound_input(
        &mut topo,
        &mut idgens,
        proc_leaf.id(),
        InputOptions::Synchronous,
        vec![SoundInputBranchId::new(1)],
    );

    topo.connect_sound_input(input_leaf_a, proc_root.id())
        .unwrap();
    topo.connect_sound_input(input_leaf_b, proc_side.id())
        .unwrap();
    topo.connect_sound_input(input_side, proc_root.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}
