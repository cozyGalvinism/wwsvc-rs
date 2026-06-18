use std::collections::HashMap;

/// A collection of parameters for a request
#[derive(Debug, Clone, Default)]
pub struct Parameters {
    inner: HashMap<String, String>,
}

impl Parameters {
    /// Creates a new empty collection of parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a parameter to the collection
    pub fn param<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.inner.insert(key.into(), value.into());
        self
    }

    /// Adds multiple parameters to the collection
    pub fn extend<I, K, V>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in iter {
            self.inner.insert(k.into(), v.into());
        }
        self
    }

    /// Returns the inner `HashMap` of parameters
    pub fn into_inner(self) -> HashMap<String, String> {
        self.inner
    }

    /// Returns a reference to the inner `HashMap` of parameters
    pub fn as_inner(&self) -> &HashMap<String, String> {
        &self.inner
    }
}

impl From<HashMap<&str, &str>> for Parameters {
    fn from(value: HashMap<&str, &str>) -> Self {
        Self {
            inner: value.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl<K, V> FromIterator<(K, V)> for Parameters
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
        }
    }
}

// Implement From for arrays to support the collection! macro
impl<const N: usize> From<[(&str, &str); N]> for Parameters {
    fn from(arr: [(&str, &str); N]) -> Self {
        Self {
            inner: arr.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }
    }
}

impl<const N: usize> From<[(String, String); N]> for Parameters {
    fn from(arr: [(String, String); N]) -> Self {
        Self {
            inner: arr.into_iter().collect()
        }
    }
}