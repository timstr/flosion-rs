use crate::{
    core::sound::{
        sounderror::SoundError,
        soundgraph::SoundGraph,
        soundgraphdata::SoundInputBranchId,
        soundgraphvalidation::find_sound_error,
        soundinput::InputOptions,
        test::testobjects::{TestDynamicSoundProcessor, TestStaticSoundProcessor},
    },
    ui_core::arguments::ParsedArguments,
};

#[test]
fn find_error_empty_graph() {
    let topo = SoundGraph::new();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_one_static_proc() {
    let mut topo = SoundGraph::new();
    topo.add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_one_dynamic_proc() {
    let mut topo = SoundGraph::new();
    topo.add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();
    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_self_cycle() {
    let mut topo = SoundGraph::new();

    let proc = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(proc.id(), InputOptions::Synchronous, Vec::new())
        .unwrap();

    topo.connect_sound_input(input_id, proc.id()).unwrap();

    assert!(match find_sound_error(&topo) {
        Some(SoundError::CircularDependency { cycle: _ }) => true,
        _ => false,
    });
}

#[test]
fn find_error_static_to_dynamic_no_branches() {
    let mut topo = SoundGraph::new();

    let static_proc = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let dynamic_proc = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(static_proc.id(), InputOptions::Synchronous, vec![])
        .unwrap();

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_dynamic_one_branch() {
    let mut topo = SoundGraph::new();

    let static_proc = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let dynamic_proc = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            static_proc.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_dynamic_two_branches() {
    let mut topo = SoundGraph::new();

    let static_proc = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let dynamic_proc = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            static_proc.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, dynamic_proc.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_no_branches() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_static_to_static_one_branch() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_two_branches() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_static_to_dynamic_one_branch_nonsync() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::NonSynchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_static_to_static_one_branch_nonsync() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::NonSynchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_no_branches() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_one_branch() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_to_static_two_branches() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_static_nonsync() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_id = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::NonSynchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_id, proc2.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc2.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_no_branches() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc3 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input1 = topo
        .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
        .unwrap();

    let input2 = topo
        .add_sound_input(
            proc2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_one_branch() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc3 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input1 = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input2 = topo
        .add_sound_input(
            proc2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_cycle() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc3 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input1 = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input2 = topo
        .add_sound_input(
            proc2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input3 = topo
        .add_sound_input(
            proc3.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

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
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc3 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input1 = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
        )
        .unwrap();

    let input2 = topo
        .add_sound_input(
            proc2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotOneState(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_nonsync() {
    let mut topo = SoundGraph::new();

    let proc1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc3 = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input1 = topo
        .add_sound_input(
            proc1.id(),
            InputOptions::NonSynchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input2 = topo
        .add_sound_input(
            proc2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input1, proc2.id()).unwrap();
    topo.connect_sound_input(input2, proc3.id()).unwrap();

    assert_eq!(
        find_sound_error(&topo),
        Some(SoundError::StaticNotSynchronous(proc3.id()))
    );
}

#[test]
fn find_error_dynamic_indirect_fork_to_static_nonsync() {
    let mut topo = SoundGraph::new();

    let proc_root1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_root2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_middle = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_leaf = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_root1 = topo
        .add_sound_input(
            proc_root1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input_root2 = topo
        .add_sound_input(
            proc_root2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input_middle = topo
        .add_sound_input(
            proc_middle.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

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
    let mut topo = SoundGraph::new();

    let proc_root1 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_root2 = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_leaf = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_root1 = topo
        .add_sound_input(
            proc_root1.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input_root2 = topo
        .add_sound_input(
            proc_root2.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_root1, proc_leaf.id())
        .unwrap();
    topo.connect_sound_input(input_root2, proc_leaf.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}

#[test]
fn find_error_dynamic_split_to_static_two_inputs() {
    let mut topo = SoundGraph::new();

    let proc_root = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_side = topo
        .add_dynamic_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_leaf = topo
        .add_static_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let input_side = topo
        .add_sound_input(
            proc_side.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input_leaf_a = topo
        .add_sound_input(
            proc_leaf.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    let input_leaf_b = topo
        .add_sound_input(
            proc_leaf.id(),
            InputOptions::Synchronous,
            vec![SoundInputBranchId::new(1)],
        )
        .unwrap();

    topo.connect_sound_input(input_leaf_a, proc_root.id())
        .unwrap();
    topo.connect_sound_input(input_leaf_b, proc_side.id())
        .unwrap();
    topo.connect_sound_input(input_side, proc_root.id())
        .unwrap();

    assert_eq!(find_sound_error(&topo), None);
}
