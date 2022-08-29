use std::net::SocketAddr;

use crate::{
    advertise::{Advertiser, ThingType},
    hlist::*,
    servient::Servient,
};
use axum::{handler::Handler, routing::MethodRouter, Router};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wot_td::{
    builder::{FormBuilder, ThingBuilder},
    extend::ExtendableThing,
};

#[doc(hidden)]
/// ThingBuilder ExtendableThing used to build a Servient via HttpRouter methods.
///
/// It is not needed to know about it nor use it.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ServientExtension {
    /// Listening address
    #[serde(skip)]
    addr: Option<SocketAddr>,
    /// Thing type
    #[serde(skip)]
    thing_type: ThingType,
}

#[doc(hidden)]
/// Form Extension
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Form {
    #[serde(skip)]
    method_router: MethodRouter,
}

impl From<MethodRouter> for Form {
    fn from(method_router: MethodRouter) -> Self {
        Self { method_router }
    }
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

/// Extension trait for the [Servient] configuration.
pub trait ServientSettings {
    /// Bind the http server to addr
    fn http_bind(self, addr: SocketAddr) -> Self;
    /// Set the thing type to be advertised.
    fn thing_type(self, ty: ThingType) -> Self;
}

impl<O: ExtendableThing> ServientSettings for ThingBuilder<O, wot_td::builder::Extended>
where
    O: Holder<ServientExtension>,
{
    fn http_bind(mut self, addr: SocketAddr) -> Self {
        self.other.field_mut().addr = Some(addr);
        self
    }

    fn thing_type(mut self, ty: ThingType) -> Self {
        self.other.field_mut().thing_type = ty;
        self
    }
}

/// Trait extension to build a [Servient] from an extended [ThingBuilder]
///
/// TODO: Add an example
pub trait BuildServient {
    type Other: ExtendableThing;
    /// Build the configured Servient
    fn build_servient(self) -> Result<Servient<Self::Other>, Box<dyn std::error::Error>>;
}

impl<O: ExtendableThing> BuildServient for ThingBuilder<O, wot_td::builder::Extended>
where
    O: Holder<ServientExtension>,
    O::Form: Holder<Form>,
    O: Serialize,
{
    type Other = O;

    /// Build the configured Servient
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

        let sd = Advertiser::new()?;

        let name = {
            let name = thing
                .title
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            let uuid = Uuid::new_v4();

            format!("{}{}", name, uuid.as_simple())
        };

        let http_addr = thing
            .other
            .field_ref()
            .addr
            .unwrap_or_else(|| "0.0.0.0:8080".parse().unwrap());

        let thing_type = thing.other.field_ref().thing_type;

        Ok(Servient {
            name,
            thing,
            router,
            sd,
            http_addr,
            thing_type,
        })
    }
}

pub trait HttpRouter {
    type Target;
    /// Route GET requests to the given handler.
    fn http_get<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    /// Route PUT requests to the given handler.
    fn http_put<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    /// Route POST requests to the given handler.
    fn http_post<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    /// Route PATCH requests to the given handler.
    fn http_patch<H, T>(self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static;
    /// Route DELETE requests to the given handler.
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

    /// Route GET requests to the given handler.
    fn http_get<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.get(handler);
        self
    }
    /// Route PUT requests to the given handler.
    fn http_put<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.put(handler);
        self
    }
    /// Route POST requests to the given handler.
    fn http_post<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.post(handler);
        self
    }
    /// Route PATCH requests to the given handler.
    fn http_patch<H, T>(mut self, handler: H) -> Self::Target
    where
        H: Handler<T, axum::body::Body>,
        T: 'static,
    {
        let method_router = std::mem::take(&mut self.other.field_mut().method_router);
        self.other.field_mut().method_router = method_router.patch(handler);
        self
    }
    /// Route DELETE requests to the given handler.
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