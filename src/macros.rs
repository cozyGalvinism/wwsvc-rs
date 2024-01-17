/// Generates a response struct with a container struct.
///
/// ## Example
///
/// Generating a response struct for an IDB GET request:
///
/// ```
/// use wwsvc_rs::generate_get_response;
///
/// generate_get_response!(TrackingResponse, "IDBID0026LISTE", TrackingListe, "IDBID0026");
/// ```
#[macro_export]
macro_rules! generate_get_response {
    ($name:ident, $container_name:literal, $container_type:ident, $list_name:literal) => {
        /// Generic response struct for a WWSVC GET request.
        #[derive(serde::Deserialize, Clone)]
        pub struct $name<T> {
            /// The COMRESULT of the request. Contains information about the status of the request.
            #[serde(rename = "COMRESULT")]
            pub com_result: $crate::responses::ComResult,
            /// The container struct for the list of items.
            #[serde(rename = $container_name)]
            pub container: $container_type<T>,
        }

        /// Container struct for the list of items.
        #[derive(serde::Deserialize, Clone)]
        pub struct $container_type<T> {
            /// The list of items.
            #[serde(rename = $list_name)]
            pub list: Option<Vec<T>>,
        }
    };
}

/// Generates a collection with syntactic sugar for vecs, sets and maps.
///
/// ## Example
///
/// ```
/// use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
/// use wwsvc_rs::collection;
///
/// let s: Vec<_> = collection![1, 2, 3];
/// println!("{:?}", s);
/// let s: BTreeSet<_> = collection!{ 1, 2, 3 };
/// println!("{:?}", s);
/// let s: HashSet<_> = collection!{ 1, 2, 3 };
/// println!("{:?}", s);
/// let s: BTreeMap<_, _> = collection!{ 1 => 2, 3 => 4 };
/// println!("{:?}", s);
/// let s: HashMap<_, _> = collection!{ 1 => 2, 3 => 4 };
/// println!("{:?}", s);
#[macro_export]
macro_rules! collection {
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k, $v),)*])
    }};
    ($($v:expr),* $(,)?) => {{
        core::convert::From::from([$($v,)*])
    }};
}
