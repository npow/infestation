#[derive(Clone, Copy)]
pub(super) struct NeverCompared<T>(pub(super) T);

impl<T> PartialEq for NeverCompared<T> {
    fn eq(&self, _other: &Self) -> bool {
        panic!("NeverCompared should never be compared");
    }
}

impl<T> Eq for NeverCompared<T> {}

impl<T> PartialOrd for NeverCompared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for NeverCompared<T> {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        panic!("NeverCompared should never be compared");
    }
}
