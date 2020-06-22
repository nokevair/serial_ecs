struct ComponentIdx {
    // identifies the type of component
    id: u16,
    // the index of the component itself
    idx: usize,
}

struct EntityData {
    components: Vec<ComponentIdx>,
}
