use crate::{
    core::number::{numbergraph::NumberGraphInputId, numbersource::NumberSourceId},
    ui_core::lexicallayout::{
        ASTNode, ASTNodeValue, ASTPath, InternalASTNode, InternalASTNodeValue,
    },
};

#[test]
fn test_get_along_path() {
    let tree = InternalASTNode::new(InternalASTNodeValue::Function(
        NumberSourceId::new(1),
        vec![
            ASTNode::new(ASTNodeValue::Empty),
            ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                InternalASTNodeValue::Function(
                    NumberSourceId::new(2),
                    vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                ),
            )))),
            ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
            ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
        ],
    ));

    let Some(tree_at_empty_path) = tree.get_along_path(&[]) else {
        panic!();
    };

    assert!(
        if let InternalASTNodeValue::Function(_, _) = tree_at_empty_path.value() {
            true
        } else {
            false
        }
    );

    assert!(tree.get_along_path(&[0]).is_none());

    let Some(tree_at_path_1) = tree.get_along_path(&[1]) else {
        panic!();
    };

    assert!(
        if let InternalASTNodeValue::Function(_, _) = tree_at_path_1.value() {
            true
        } else {
            false
        }
    );

    assert!(tree.get_along_path(&[2]).is_none());

    assert!(tree.get_along_path(&[3]).is_none());
}

#[test]
fn test_go_left() {
    let tree = InternalASTNode::new(InternalASTNodeValue::Function(
        NumberSourceId::new(1),
        vec![
            ASTNode::new(ASTNodeValue::Empty),
            ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                InternalASTNodeValue::Function(
                    NumberSourceId::new(2),
                    vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                ),
            )))),
            ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
            ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
        ],
    ));

    let mut path = ASTPath::new(vec![]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![1, 0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![3]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[2]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1, 0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);
}

#[test]
fn test_go_right() {
    let tree = InternalASTNode::new(InternalASTNodeValue::Function(
        NumberSourceId::new(1),
        vec![
            ASTNode::new(ASTNodeValue::Empty),
            ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                InternalASTNodeValue::Function(
                    NumberSourceId::new(2),
                    vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                ),
            )))),
            ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
            ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
        ],
    ));

    let mut path = ASTPath::new(vec![]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[1, 0]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[2,]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[3]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[3]);
}
