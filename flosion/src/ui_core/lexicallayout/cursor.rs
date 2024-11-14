use eframe::egui;
use hashstash::{Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use super::{
    ast::{ASTNode, ASTPath, FinalExpression, VariableDefinition},
    lexicallayout::LexicalLayout,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum LineLocation {
    VariableDefinition(usize),
    FinalExpression(usize),
}

pub(super) enum LexicalLayoutCursorValue<'a> {
    AtVariableName(&'a VariableDefinition),
    AtVariableValue(&'a VariableDefinition, ASTPath),
    AtFinalExpression(&'a FinalExpression, ASTPath),
}

#[derive(Clone)]
pub(crate) enum LexicalLayoutCursor {
    AtVariableName(usize),
    AtVariableValue(usize, ASTPath),
    AtFinalExpression(usize, ASTPath),
}

impl LexicalLayoutCursor {
    pub(super) fn line(&self) -> LineLocation {
        match self {
            LexicalLayoutCursor::AtVariableName(l) => LineLocation::VariableDefinition(*l),
            LexicalLayoutCursor::AtVariableValue(l, _) => LineLocation::VariableDefinition(*l),
            LexicalLayoutCursor::AtFinalExpression(l, _) => LineLocation::FinalExpression(*l),
        }
    }

    pub(super) fn path_mut(&mut self) -> Option<&mut ASTPath> {
        match self {
            LexicalLayoutCursor::AtVariableName(_) => None,
            LexicalLayoutCursor::AtVariableValue(_, p) => Some(p),
            LexicalLayoutCursor::AtFinalExpression(_, p) => Some(p),
        }
    }

    pub(super) fn go_left(&mut self, layout: &LexicalLayout) {
        let defns = layout.variable_definitions();
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                let Some(i_prev) = i.checked_sub(1) else {
                    return;
                };
                *self = LexicalLayoutCursor::AtVariableValue(
                    i_prev,
                    ASTPath::new_at_end_of(defns[i_prev].value()),
                );
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                if p.is_at_beginning() {
                    *self = LexicalLayoutCursor::AtVariableName(*i);
                } else {
                    p.go_left(defns[*i].value());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(i_expr, p) => {
                if p.is_at_beginning() {
                    if *i_expr > 0 {
                        let new_i_expr = *i_expr - 1;
                        *self = LexicalLayoutCursor::AtFinalExpression(
                            new_i_expr,
                            ASTPath::new_at_end_of(layout.final_expressions()[new_i_expr].value()),
                        )
                    }
                    let Some(i) = defns.len().checked_sub(1) else {
                        return;
                    };
                    *self = LexicalLayoutCursor::AtVariableValue(
                        i,
                        ASTPath::new_at_end_of(defns[i].value()),
                    );
                } else {
                    p.go_left(layout.final_expressions()[*i_expr].value());
                }
            }
        }
    }

    pub(super) fn go_right(&mut self, layout: &LexicalLayout) {
        let defns = layout.variable_definitions();
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                *self = LexicalLayoutCursor::AtVariableValue(*i, ASTPath::new_at_beginning());
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                if p.is_at_end_of(defns[*i].value()) {
                    let i_next = *i + 1;
                    if i_next == defns.len() {
                        *self =
                            LexicalLayoutCursor::AtFinalExpression(0, ASTPath::new_at_beginning());
                    } else {
                        *self = LexicalLayoutCursor::AtVariableName(i_next);
                    }
                } else {
                    p.go_right(defns[*i].value());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(i_expr, p) => {
                if p.is_at_end_of(layout.final_expressions()[*i_expr].value()) {
                    let next_i_expr = *i_expr + 1;
                    if next_i_expr < layout.final_expressions().len() {
                        *self = LexicalLayoutCursor::AtFinalExpression(
                            next_i_expr,
                            ASTPath::new_at_beginning(),
                        );
                    }
                } else {
                    p.go_right(layout.final_expressions()[*i_expr].value());
                }
            }
        }
    }

    pub(super) fn go_up(&mut self, layout: &LexicalLayout) {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                *self = LexicalLayoutCursor::AtVariableName(i.saturating_sub(1))
            }
            LexicalLayoutCursor::AtVariableValue(i, _) => {
                *self = LexicalLayoutCursor::AtVariableValue(
                    i.saturating_sub(1),
                    ASTPath::new_at_beginning(),
                );
            }
            LexicalLayoutCursor::AtFinalExpression(i_expr, _) => {
                if *i_expr > 0 {
                    *self = LexicalLayoutCursor::AtFinalExpression(
                        *i_expr - 1,
                        ASTPath::new_at_beginning(),
                    );
                    return;
                }
                let Some(i) = layout.variable_definitions().len().checked_sub(1) else {
                    *self = LexicalLayoutCursor::AtFinalExpression(0, ASTPath::new_at_beginning());
                    return;
                };
                *self = LexicalLayoutCursor::AtVariableValue(i, ASTPath::new_at_beginning());
            }
        }
    }

    pub(super) fn go_down(&mut self, layout: &LexicalLayout) {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                let i_next = *i + 1;
                if i_next == layout.variable_definitions().len() {
                    *self = LexicalLayoutCursor::AtFinalExpression(0, ASTPath::new_at_beginning());
                } else {
                    *self = LexicalLayoutCursor::AtVariableName(i_next);
                }
            }
            LexicalLayoutCursor::AtVariableValue(i, _) => {
                let i_next = *i + 1;
                if i_next == layout.variable_definitions().len() {
                    *self = LexicalLayoutCursor::AtFinalExpression(0, ASTPath::new_at_beginning());
                } else {
                    *self =
                        LexicalLayoutCursor::AtVariableValue(i_next, ASTPath::new_at_beginning());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(i, _) => {
                let i_next = *i + 1;
                if i_next < layout.final_expressions().len() {
                    *self =
                        LexicalLayoutCursor::AtFinalExpression(i_next, ASTPath::new_at_beginning());
                } else {
                    *self = LexicalLayoutCursor::AtFinalExpression(
                        *i,
                        ASTPath::new_at_end_of(layout.final_expressions()[*i].value()),
                    );
                }
            }
        }
    }

    pub(super) fn get<'a>(&self, layout: &'a LexicalLayout) -> LexicalLayoutCursorValue<'a> {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                LexicalLayoutCursorValue::AtVariableName(&layout.variable_definitions()[*i])
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                LexicalLayoutCursorValue::AtVariableValue(
                    &layout.variable_definitions()[*i],
                    p.clone(),
                )
            }
            LexicalLayoutCursor::AtFinalExpression(i, p) => {
                LexicalLayoutCursorValue::AtFinalExpression(
                    &layout.final_expressions()[*i],
                    p.clone(),
                )
            }
        }
    }

    pub(crate) fn get_bounding_rect(&self, layout: &LexicalLayout) -> Option<egui::Rect> {
        match self {
            LexicalLayoutCursor::AtVariableName(var_idx) => layout
                .variable_definitions()
                .get(*var_idx)
                .map(|vd| vd.name_rect()),
            _ => self.get_node(layout).map(|n| n.rect()),
        }
    }

    pub(super) fn get_node<'a>(&self, layout: &'a LexicalLayout) -> Option<&'a ASTNode> {
        match self {
            LexicalLayoutCursor::AtVariableName(_) => None,
            LexicalLayoutCursor::AtVariableValue(i, p) => Some(
                layout.variable_definitions()[*i]
                    .value()
                    .get_along_path(p.steps()),
            ),
            LexicalLayoutCursor::AtFinalExpression(i, p) => Some(
                layout.final_expressions()[*i]
                    .value()
                    .get_along_path(p.steps()),
            ),
        }
    }

    pub(super) fn get_variables_in_scope<'a>(
        &self,
        layout: &'a LexicalLayout,
    ) -> &'a [VariableDefinition] {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => &layout.variable_definitions()[..(*i)],
            LexicalLayoutCursor::AtVariableValue(i, _) => &layout.variable_definitions()[..(*i)],
            LexicalLayoutCursor::AtFinalExpression(_, _) => layout.variable_definitions(),
        }
    }

    pub(super) fn set_node(&self, layout: &mut LexicalLayout, value: ASTNode) {
        let (node, path) = match self {
            LexicalLayoutCursor::AtVariableName(_) => {
                panic!("Can't set a node value when the cursor is pointing at a variable name")
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                (layout.variable_definitions_mut()[*i].value_mut(), p)
            }
            LexicalLayoutCursor::AtFinalExpression(i, p) => {
                (layout.final_expressions_mut()[*i].value_mut(), p)
            }
        };

        node.set_along_path(path.steps(), value);
    }
}

impl Stashable for LexicalLayoutCursor {
    fn stash(&self, stasher: &mut Stasher) {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                stasher.u8(0);
                stasher.u64(*i as _);
            }
            LexicalLayoutCursor::AtVariableValue(i, astpath) => {
                stasher.u8(1);
                stasher.u64(*i as _);
                astpath.stash(stasher);
            }
            LexicalLayoutCursor::AtFinalExpression(i, astpath) => {
                stasher.u8(2);
                stasher.u64(*i as _);
                astpath.stash(stasher);
            }
        }
    }
}

impl Unstashable for LexicalLayoutCursor {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let cursor = match unstasher.u8()? {
            0 => LexicalLayoutCursor::AtVariableName(unstasher.u64()? as _),
            1 => LexicalLayoutCursor::AtVariableValue(
                unstasher.u64()? as _,
                ASTPath::unstash(unstasher)?,
            ),
            2 => LexicalLayoutCursor::AtFinalExpression(
                unstasher.u64()? as _,
                ASTPath::unstash(unstasher)?,
            ),
            _ => panic!(),
        };
        Ok(cursor)
    }
}
