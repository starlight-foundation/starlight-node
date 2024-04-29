use crate::keys::Public;

pub enum Command {
    AccountBalance(Public),
    Version
}