//! Path parameter extraction layer — parses route placeholders into layer Data.

use std::future::Future;

use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::tag::Tagged;

/// Tag for the path-parameter map in layer Data.
pub struct PathTag;

/// Named path parameters from the route.
#[derive(Clone, Default, Debug)]
pub struct PathParams {
    segments: Vec<(String, String)>,
}

impl PathParams {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.segments
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl ToString) {
        let name = name.into();
        let value = value.to_string();
        if let Some(entry) = self.segments.iter_mut().find(|(k, _)| *k == name) {
            entry.1 = value;
        } else {
            self.segments.push((name, value));
        }
    }
}

/// Extracts named path parameters from [`LayerRequest::path`] into Data.
#[derive(Clone, Copy, Debug)]
pub struct PathLayer {
    pub names: &'static [&'static str],
}

impl PathLayer {
    pub const fn names(names: &'static [&'static str]) -> Self {
        Self { names }
    }

    pub const fn all() -> Self {
        Self { names: &[] }
    }
}

impl LayerContrib for PathLayer {
    type Contrib = HCons<Tagged<PathTag, PathParams>, HNil>;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for PathLayer
where
    Acc: HList + Send,
    Ctx: Send,
{
    type AccOut = HCons<Tagged<PathTag, PathParams>, Acc>;

    fn run<'a>(
        &'a self,
        _ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a,
    {
        async move {
            let map = if self.names.is_empty() {
                req.path.clone()
            } else {
                let mut m = PathParams::default();
                for name in self.names {
                    if let Some(v) = req.path.get(name) {
                        m.insert(*name, v);
                    }
                }
                m
            };
            LayerStep::Continue(cons_tagged::<PathTag, _, _>(map, acc))
        }
    }
}
