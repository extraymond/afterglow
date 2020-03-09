use crate::prelude::*;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

pub trait Link {
    type Local;
    type Remote;

    fn meet();
}
