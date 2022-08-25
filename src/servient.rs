use axum::{handler::Handler, routing::MethodRouter, Router};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use wot_td::{
    builder::{FormBuilder, ThingBuilder, ToExtend},
    extend::ExtendableThing,
    hlist::*,
    thing::Thing,
};

use crate::hlist::*;

/// WoT Servient serving a Thing Description
pub struct Servient<Other: ExtendableThing = Nil> {
    pub thing: Thing<Other>,
    pub router: Router,
}

impl Servient<Nil> {
    pub fn builder(title: impl Into<String>) -> ThingBuilder<NilPlus<ServientExtension>, ToExtend> {
        ThingBuilder::<NilPlus<ServientExtension>, ToExtend>::new(title)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ServientExtension {}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Form {
    #[serde(skip)]
    method_router: MethodRouter,
}

impl ExtendableThing for ServientExtension {
    type InteractionAffordance = ();
    type PropertyAffordance = ();
    type ActionAffordance = ();
    type EventAffordance = ();
    type Form = Form;
    type ExpectedResponse = ();
    type DataSchema = ();
    type ObjectSchema = ();
    type ArraySchema = ();
}

pub trait BuildServient {
    type Other: ExtendableThing;
    fn build_servient(self) -> Result<Servient<Self::Other>, Box<dyn std::error::Error>>;
}

impl<O: ExtendableThing> BuildServient for ThingBuilder<O, wot_td::builder::Extended>
where
    O: Holder<ServientExtension>,
    O::Form: Holder<Form>,
    O: Serialize,
{
    type Other = O;

    fn build_servient(self) -> Result<Servient<Self::Other>, Box<dyn std::error::Error>> {
        let thing = self.build()?;

        let mut router = Router::new();

        let thing_forms = thing.forms.iter().flat_map(|o| o.iter());
        let properties_forms = thing
            .properties
            .iter()
            .flat_map(|m| m.values().flat_map(|a| a.interaction.forms.iter()));
        let actions_forms = thing
            .actions
            .iter()
            .flat_map(|m| m.values().flat_map(|a| a.interaction.forms.iter()));
        let events_forms = thing
            .events
            .iter()
            .flat_map(|m| m.values().flat_map(|a| a.interaction.forms.iter()));

        for form in thing_forms
            .chain(properties_forms)
            .chain(actions_forms)
            .chain(events_forms)
        {
            let route = form.other.field_ref();

            router = router.route(&form.href, route.method_router.clone());
        }

        // TODO: Figure out how to share the thing description and if we want to.
        let json = serde_json::to_value(&thing)?;

        router = router.route(
            "/.well-known/wot",
            axum::routing::get(move || async { axum::Json(json) }),
        );

        Ok(Servient { thing, router })
    }
}

pub trait HttpRouter {
    type Target;
    fn http_get<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    fn http_put<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    fn http_post<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    fn http_patch<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    fn http_delete<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
}

impl<Other, Href, OtherForm> HttpRouter for FormBuilder<Other, Href, OtherForm>
where
    Other: ExtendableThing + Holder<ServientExtension>,
    OtherForm: Holder<Form>,
{
    type Target = FormBuilder<Other, Href, OtherForm>;
    fn http_get<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.get(handler);
        self
    }
    fn http_put<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.put(handler);
        self
    }
    fn http_post<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.post(handler);
        self
    }
    fn http_patch<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.patch(handler);
        self
    }
    fn http_delete<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.delete(handler);
        self
    }
}

#[cfg(test)]
mod test {
    use wot_td::{builder::affordance::*, builder::data_schema::*, thing::FormOperation};

    use crate::servient::HttpRouter;

    use super::{BuildServient, Servient};

    #[test]
    fn build_servient() {
        let servient = Servient::builder("test")
            .finish_extend()
            .form(|f| {
                f.href("/ref")
                    .http_get(|| async { "Hello, World!" })
                    .op(FormOperation::ReadAllProperties)
            })
            .form(|f| {
                f.href("/ref2")
                    .http_get(|| async { "Hello, World! 2" })
                    .op(FormOperation::ReadAllProperties)
            })
            .build_servient()
            .unwrap();

        dbg!(&servient.router);
    }

    #[test]
    fn build_servient_property() {
        let servient = Servient::builder("test")
            .finish_extend()
            .property("hello", |b| {
                b.finish_extend_data_schema().null().form(|f| {
                    f.href("/hello")
                        .http_get(|| async { "Reading Hello, World!" })
                        .http_put(|| async { "Writing Hello, World!" })
                        .op(FormOperation::ReadProperty)
                        .op(FormOperation::WriteProperty)
                })
            })
            .build_servient()
            .unwrap();

        dbg!(&servient.router);
    }

    #[test]
    fn build_servient_action() {
        let servient = Servient::builder("test")
            .finish_extend()
            .action("hello", |b| {
                b.input(|i| i.finish_extend().number()).form(|f| {
                    f.href("/say_hello")
                        .http_post(|| async { "Saying Hello, World!" })
                })
            })
            .action("update", |b| {
                b.form(|f| {
                    f.href("/update_hello")
                        .http_patch(|| async { "Updating Hello, World!" })
                })
            })
            .action("delete", |b| {
                b.form(|f| {
                    f.href("/delete_hello")
                        .http_delete(|| async { "Goodbye, World!" })
                })
            })
            .build_servient()
            .unwrap();

        dbg!(&servient.router);
    }
}
