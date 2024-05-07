use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::ovn::components::logical_router::{ExternalGatewayMap, NatMap, RoutingMap};
use crate::ovn::configuration::external_gateway::OvnExternalGateway;
use crate::ovn::configuration::nat::OvnNat;
use crate::ovn::configuration::route::OvnRoute;
// need to implement custom serialisation implementations for the OVN configuration as they do not
// serialise without some assistance due to tuple keys

// there is a bit of serde magic going on here, but we deserialise into a list intermediate then
// make that a hashmap - and the inverse for serialise, create a list of hashmap then serialise

impl Serialize for RoutingMap
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        #[derive(Serialize)]
        struct Entry<K, V> {
            key: K,
            val: V,
        }

        serializer.collect_seq(self.0.iter().map(|(key, val)| Entry { key, val }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RouteKeyVal {
    key: (String, String, String),
    val: OvnRoute,
}

impl<'de> Deserialize<'de> for RoutingMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>
    {
        Vec::<RouteKeyVal>::deserialize(deserializer)
            .map(|mut v| RoutingMap(v.drain(..).map(|kv: RouteKeyVal | (kv.key, kv.val)).collect()))
    }
}

//

impl Serialize for ExternalGatewayMap
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        #[derive(Serialize)]
        struct Entry<K, V> {
            key: K,
            val: V,
        }

        serializer.collect_seq(self.0.iter().map(|(key, val)| Entry { key, val }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ExternalGatewayKeyVal {
    key: (String, String),
    val: OvnExternalGateway,
}

impl<'de> Deserialize<'de> for ExternalGatewayMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>
    {
        Vec::<ExternalGatewayKeyVal>::deserialize(deserializer)
            .map(|mut v| ExternalGatewayMap(v.drain(..).map(|kv: ExternalGatewayKeyVal | (kv.key, kv.val)).collect()))
    }
}

//

impl Serialize for NatMap
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        #[derive(Serialize)]
        struct Entry<K, V> {
            key: K,
            val: V,
        }

        serializer.collect_seq(self.0.iter().map(|(key, val)| Entry { key, val }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct NatKeyVal {
    key: (String, String, String),
    val: OvnNat,
}

impl<'de> Deserialize<'de> for NatMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>
    {
        Vec::<NatKeyVal>::deserialize(deserializer)
            .map(|mut v| NatMap(v.drain(..).map(|kv: NatKeyVal | (kv.key, kv.val)).collect()))
    }
}
