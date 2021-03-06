use crate::ast::TyKind;
use crate::lex::Ident;
use crate::Id;

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Item {
    Fn(ItemFn),
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct ItemFn {
    pub id: Id,
    pub args: Vec<(Ident, TyKind)>,
}
