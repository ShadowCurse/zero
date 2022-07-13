trait Binding: Default {
    type BindGroup: AssetBindGroup;
    fn bind_group_meta(bind_group: &Self::BindGroup) -> super::system::BindGroupMeta;
}

#[derive(Default)]
struct ColorMaterialBinding<const I: usize>;

impl<const I: usize> Binding for ColorMaterialBinding<I> {
    type BindGroup = ColorMaterialBindGroup;
    fn bind_group_meta(bind_group: &Self::BindGroup) -> super::system::BindGroupMeta {
        super::system::BindGroupMeta {
            index: I as u32,
            bind_group_id: bind_group.bind_group(), 
        }
    }
}

pub trait AssetBindGroup {
    type ResourceHandle;
    type BindingType;
    fn bind_group(&self) -> super::storage::ResourceId;
}

struct ColorMaterialBindGroup;

impl AssetBindGroup for ColorMaterialBindGroup {
    type ResourceHandle = ();
    type BindingType =  ();
    fn bind_group(&self) -> super::storage::ResourceId {
        super::storage::ResourceId::WINDOW_VIEW_ID
    }
}

struct Pipeline<BindingTypes> {
    _phantom: std::marker::PhantomData<BindingTypes>,
}

impl<BindingTypes> Pipeline<BindingTypes> {
    fn new() -> Self { Self{ _phantom: Default::default()  } }
    fn add_bind_group<B: Binding>(self: Pipeline<BindingTypes>) -> Pipeline<(B, BindingTypes)> {
        Pipeline {
            _phantom: Default::default(),
        }
    }
}

struct Index<const I: usize>;

trait Foo<BG: AssetBindGroup> {
    fn bind_group_meta(bind_group: &BG) -> super::system::BindGroupMeta;
}

impl<BG: AssetBindGroup, const I: usize> Foo<BG> for (BG, Index<I>) {
    fn bind_group_meta(bind_group: &BG) -> super::system::BindGroupMeta {
        super::system::BindGroupMeta {
            index: I as u32,
            bind_group_id: bind_group.bind_group(), 
        }
    }
}

fn foo() {
    let bg = ColorMaterialBindGroup;
    let meta = <(ColorMaterialBindGroup, Index<1>)>::bind_group_meta(&bg);
    let meta = ColorMaterialBinding::<1>::bind_group_meta(&bg);
    let pipeline = Pipeline::<()>::new()
        .add_bind_group::<ColorMaterialBinding::<1>>()
        .add_bind_group::<ColorMaterialBinding::<2>>();

}
