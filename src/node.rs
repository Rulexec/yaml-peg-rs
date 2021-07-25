use crate::{repr::*, *};
use alloc::string::ToString;
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::Hash,
    ops::Index,
    str::FromStr,
};

macro_rules! as_method {
    {$(#[$meta:meta])* fn $id:ident = $ty:ident$(($op:ident))?
        $(| ($default:expr)?)?
        $(| $ty2:ident)* -> $r:ty} => {
        $(#[$meta])*
        pub fn $id(&self) -> Result<$r, u64> {
            match self.yaml() {
                YamlBase::$ty(v) $(| YamlBase::$ty2(v))* => Ok(v$(.$op())?),
                $(YamlBase::Null => Ok($default),)?
                _ => Err(self.pos()),
            }
        }
    };
}

macro_rules! as_num_method {
    {$(#[$meta:meta])* fn $id:ident = $ty1:ident $(| $ty2:ident)*} => {
        $(#[$meta])*
        pub fn $id<N>(&self) -> Result<N, u64>
        where
            N: FromStr,
        {
            match self.yaml() {
                YamlBase::$ty1(n) $(| YamlBase::$ty2(n))* => match n.parse() {
                    Ok(v) => Ok(v),
                    Err(_) => Err(self.pos()),
                },
                _ => Err(self.pos()),
            }
        }
    };
}

/// A node with [`alloc::rc::Rc`] holder.
pub type Node = NodeBase<RcRepr>;
/// A node with [`alloc::sync::Arc`] holder.
pub type ArcNode = NodeBase<ArcRepr>;

/// Readonly node, including line number, column number, type assertion and anchor.
/// You can access [`YamlBase`] type through [`NodeBase::yaml`] method.
///
/// This type will ignore additional information when comparison and hashing.
///
/// ```
/// use std::collections::HashSet;
/// use yaml_peg::Node;
/// let mut s = HashSet::new();
/// s.insert(Node::new("a".into(), 0, "", ""));
/// s.insert(Node::new("a".into(), 1, "my-type", ""));
/// s.insert(Node::new("a".into(), 2, "", "my-anchor"));
/// assert_eq!(s.len(), 1);
/// ```
///
/// There is a convenient macro [`node!`] to create nodes literally.
///
/// Nodes can be indexing by `usize` or `&str`,
/// but it will always return self if the index is not contained.
///
/// ```
/// use yaml_peg::node;
/// let n = node!(null);
/// assert_eq!(n["a"][0]["bc"], n);
/// ```
///
/// There are `as_*` methods provide `Result<T, u64>` returns with node position,
/// default options can be created by [`Result::unwrap_or`],
/// additional error message can be attach by [`Result::map_err`],
/// and the optional [`Option`] can be return by [`Result::ok`],
/// which shown as following example:
///
/// ```
/// use yaml_peg::node;
///
/// fn main() -> Result<(), (&'static str, u64)> {
///     let n = node!({
///         node!("title") => node!(12.)
///     });
///     let n = n.get(&["title"]).map_err(|p| ("missing \"title\"", p))?;
///     assert_eq!(
///         Err(("title", 0)),
///         n.as_str().map_err(|p| ("title", p))
///     );
///     assert_eq!(
///         Option::<&str>::None,
///         n.as_str().ok()
///     );
///     Ok(())
/// }
/// ```
///
/// For default value on map type, [`NodeBase::get`] method has a shorten method [`NodeBase::get_default`] to combining
/// transform function and default function as well.
///
/// # Clone
///
/// Since the YAML data is wrapped by reference counter [`alloc::rc::Rc`] and [`alloc::sync::Arc`],
/// cloning node just increase the reference counter,
/// the entire data structure are still shared together.
///
/// ```
/// use std::rc::Rc;
/// use yaml_peg::node;
/// let a = node!("a");
/// {
///     let b = a.clone();
///     assert_eq!(2, Rc::strong_count(b.as_ref()));
/// }
/// assert_eq!(1, Rc::strong_count(a.as_ref()));
/// ```
///
/// If you want to copy data, please get the data first.
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct NodeBase<R: Repr>(pub(crate) R);

impl<R: Repr> NodeBase<R> {
    /// Create node from YAML data.
    pub fn new(yaml: YamlBase<R>, pos: u64, ty: &str, anchor: &str) -> Self {
        Self(R::repr(yaml, pos, ty.to_string(), anchor.to_string()))
    }

    /// Document position.
    #[inline(always)]
    pub fn pos(&self) -> u64 {
        self.0.as_ref().pos
    }

    /// Type assertion.
    #[inline(always)]
    pub fn ty(&self) -> &str {
        &self.0.as_ref().ty
    }

    /// Anchor reference.
    #[inline(always)]
    pub fn anchor(&self) -> &str {
        &self.0.as_ref().anchor
    }

    /// YAML data.
    #[inline(always)]
    pub fn yaml(&self) -> &YamlBase<R> {
        &self.0.as_ref().yaml
    }

    /// Drop the node and get the YAML data.
    pub fn into_yaml(self) -> YamlBase<R> {
        self.0.into_yaml()
    }

    /// Check the value is null.
    pub fn is_null(&self) -> bool {
        *self.yaml() == YamlBase::Null
    }

    as_method! {
        /// Convert to boolean.
        ///
        /// ```
        /// use yaml_peg::{node};
        /// assert!(node!(true).as_bool().unwrap());
        /// ```
        fn as_bool = Bool(clone) -> bool
    }

    as_num_method! {
        /// Convert to integer.
        ///
        /// ```
        /// use yaml_peg::node;
        /// assert_eq!(60, node!(60).as_int().unwrap());
        /// ```
        fn as_int = Int
    }

    as_num_method! {
        /// Convert to float.
        ///
        /// ```
        /// use yaml_peg::node;
        /// assert_eq!(20.06, node!(20.06).as_float().unwrap());
        /// ```
        fn as_float = Float
    }

    as_num_method! {
        /// Convert to number.
        ///
        /// ```
        /// use yaml_peg::node;
        /// assert_eq!(60, node!(60).as_number().unwrap());
        /// assert_eq!(20.06, node!(20.06).as_number().unwrap());
        /// ```
        fn as_number = Int | Float
    }

    as_method! {
        /// Convert to string pointer.
        ///
        /// This method allows null, it represented as empty string.
        /// You can check them by [`str::is_empty`].
        ///
        /// ```
        /// use yaml_peg::node;
        /// assert_eq!("abc", node!("abc").as_str().unwrap());
        /// assert!(node!(null).as_str().unwrap().is_empty());
        /// ```
        fn as_str = Str | ("")? -> &str
    }

    /// Convert to string pointer for string, null, bool, int, and float type.
    ///
    /// This method is useful when the option mixed with digit values.
    ///
    /// ```
    /// use yaml_peg::node;
    /// assert_eq!("abc", node!("abc").as_value().unwrap());
    /// assert_eq!("123", node!(123).as_value().unwrap());
    /// assert_eq!("12.04", node!(12.04).as_value().unwrap());
    /// assert_eq!("true", node!(true).as_value().unwrap());
    /// assert_eq!("false", node!(false).as_value().unwrap());
    /// assert!(node!(null).as_value().unwrap().is_empty());
    /// ```
    pub fn as_value(&self) -> Result<&str, u64> {
        match self.yaml() {
            YamlBase::Str(s) | YamlBase::Int(s) | YamlBase::Float(s) => Ok(s),
            YamlBase::Bool(true) => Ok("true"),
            YamlBase::Bool(false) => Ok("false"),
            YamlBase::Null => Ok(""),
            _ => Err(self.pos()),
        }
    }

    /// Infer an anchor with visitor.
    ///
    /// If the node is not a anchor, or the anchor does not exist, the original node is returned.
    /// Since the anchor type is invalid except for this method, missing anchor will still return an error.
    ///
    /// ```
    /// use yaml_peg::{node, anchors};
    /// let node_a = node!(*"a");
    /// let v = anchors!["a" => node!(20.)];
    /// assert_eq!(20., node_a.as_anchor(&v).as_float().unwrap());
    /// ```
    pub fn as_anchor(&self, anchors: &AnchorBase<R>) -> Self {
        match self.yaml() {
            YamlBase::Anchor(s) if anchors.contains_key(s) => anchors.get(s).unwrap().clone(),
            _ => self.clone(),
        }
    }

    as_method! {
        /// Convert to array. The object ownership will be took.
        ///
        /// ```
        /// use yaml_peg::node;
        /// let n = node!([node!(1), node!(2)]);
        /// assert_eq!(&vec![node!(1), node!(2)], n.as_array().unwrap());
        /// let n = node!([node!("55")]);
        /// for n in n.as_array().unwrap() {
        ///     assert_eq!(&node!("55"), n);
        /// }
        /// ```
        fn as_array = Array -> &Array<R>
    }

    as_method! {
        /// Convert to map. The object ownership will be took.
        ///
        /// ```
        /// use yaml_peg::node;
        /// let n = node!({node!(1) => node!(2)});
        /// assert_eq!(node!(2), n.as_map().unwrap()[&node!(1)]);
        /// for (k, v) in n.as_map().unwrap() {
        ///     assert_eq!(&node!(1), k);
        ///     assert_eq!(&node!(2), v);
        /// }
        /// ```
        fn as_map = Map -> &Map<R>
    }

    /// Convert to map and try to get the value by keys recursivly.
    ///
    /// If any key is missing, return `Err` with node position.
    ///
    /// ```
    /// use yaml_peg::node;
    /// let n = node!({node!("a") => node!({node!("b") => node!(30.)})});
    /// assert_eq!(node!(30.), n.get(&["a", "b"]).unwrap());
    /// ```
    pub fn get<Y>(&self, keys: &[Y]) -> Result<Self, u64>
    where
        Y: Into<YamlBase<R>> + Copy,
    {
        if keys.is_empty() {
            panic!("invalid search!");
        }
        match self.yaml() {
            YamlBase::Map(m) => {
                if let Some(n) = m.get(&keys[0].into().into()) {
                    if keys[1..].is_empty() {
                        Ok(n.clone())
                    } else {
                        n.get(&keys[1..])
                    }
                } else {
                    Err(self.pos())
                }
            }
            _ => Err(self.pos()),
        }
    }

    /// Same as [`NodeBase::get`] but provide default value if the key is missing.
    /// For this method, a transform method `as_*` is required.
    ///
    /// + If the value exist, return the value.
    /// + If value is a wrong type, return `Err` with node position.
    /// + If the value is not exist, return the default value.
    ///
    /// ```
    /// use yaml_peg::{node, NodeBase};
    /// let a = node!({node!("a") => node!({node!("b") => node!("c")})});
    /// assert_eq!(
    ///     "c",
    ///     a.get_default(&["a", "b"], "d", NodeBase::as_str).unwrap()
    /// );
    /// let b = node!({node!("a") => node!({})});
    /// assert_eq!(
    ///     "d",
    ///     b.get_default(&["a", "b"], "d", NodeBase::as_str).unwrap()
    /// );
    /// let c = node!({node!("a") => node!({node!("b") => node!(20.)})});
    /// assert_eq!(
    ///     Err(0),
    ///     c.get_default(&["a", "b"], "d", NodeBase::as_str)
    /// );
    /// ```
    pub fn get_default<'a, Y, Ret, F>(
        &'a self,
        keys: &[Y],
        default: Ret,
        factory: F,
    ) -> Result<Ret, u64>
    where
        Y: Into<YamlBase<R>> + Copy,
        F: Fn(&'a Self) -> Result<Ret, u64>,
    {
        if keys.is_empty() {
            panic!("invalid search!");
        }
        match self.yaml() {
            YamlBase::Map(m) => {
                if let Some(n) = m.get(&keys[0].into().into()) {
                    if keys[1..].is_empty() {
                        factory(n)
                    } else {
                        n.get_default(&keys[1..], default, factory)
                    }
                } else {
                    Ok(default)
                }
            }
            _ => Err(self.pos()),
        }
    }
}

impl<R: Repr> Debug for NodeBase<R> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_fmt(format_args!("Node{:?}", &self.0))
    }
}

impl<R: Repr> Index<usize> for NodeBase<R> {
    type Output = Self;

    fn index(&self, index: usize) -> &Self::Output {
        match self.yaml() {
            YamlBase::Array(a) => a.get(index).unwrap_or(self),
            YamlBase::Map(m) => m
                .get(&YamlBase::Int(index.to_string()).into())
                .unwrap_or(self),
            _ => self,
        }
    }
}

impl<R: Repr> Index<&str> for NodeBase<R> {
    type Output = Self;

    fn index(&self, index: &str) -> &Self::Output {
        if let YamlBase::Map(m) = self.yaml() {
            m.get(&YamlBase::from(index).into()).unwrap_or(self)
        } else {
            self
        }
    }
}

impl<R: Repr> From<YamlBase<R>> for NodeBase<R> {
    fn from(yaml: YamlBase<R>) -> Self {
        Self::new(yaml, 0, "", "")
    }
}
