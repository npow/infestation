use enum_map::Enum;

pub(crate) trait EnumAll: Enum {
    fn iter_all() -> impl Iterator<Item = Self>;
}

impl<T: Enum> EnumAll for T {
    fn iter_all() -> impl Iterator<Item = Self> {
        (0..Self::LENGTH).map(Self::from_usize)
    }
}
