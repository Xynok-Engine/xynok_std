#![allow(unused)]
use std::marker::PhantomData;

// State types
#[derive(Debug)]
struct Launch;
#[derive(Debug)]
struct Application;
#[derive(Debug)]
struct Platform;
#[derive(Debug)]
struct Rendering;
#[derive(Debug)]
struct Editor;
#[derive(Debug)]
struct Resource;

#[derive(Debug)]
enum FeaturePlatform
{
    Winit,
    Vulkan,
    KeyboardAndMouse,
    Controller,
}
#[derive(Debug)]
enum FeatureRendering
{
    Srp,
    Urp,
    Hrp,
}
#[derive(Debug)]
enum FeatureApplication
{
    HotReload,
    RunInBackground,
}

#[derive(Debug, Default)]
struct EngineData
{
    platform_features:    Vec<FeaturePlatform>,
    render_features:      Vec<FeatureRendering>,
    application_features: Vec<FeatureApplication>,
}

#[derive(Debug)]
struct Engine<State>
where State: std::fmt::Debug
{
    data:   EngineData,
    _state: PhantomData<State>,
}

impl Engine<Launch>
{
    fn new() -> Engine<Platform>
    {
        println!("Engine: Launched !");
        Engine { data: EngineData::default(), _state: PhantomData }
    }
}
impl Engine<Platform>
{
    fn init(mut self) -> Engine<Resource>
    {
        self.data.platform_features.push(FeaturePlatform::Winit);
        println!("Engine: Platform initialized !");
        Engine { data: self.data, _state: PhantomData }
    }
}

impl Engine<Resource>
{
    fn setup_resource(mut self) -> Engine<Rendering>
    {
        println!("Engine: Resource initialized !");
        Engine { data: self.data, _state: PhantomData }
    }
}
impl Engine<Rendering>
{
    fn setup_rendering(mut self) -> Engine<Editor>
    {
        self.data.render_features.push(FeatureRendering::Srp);
        self.data.render_features.push(FeatureRendering::Urp);
        self.data.render_features.push(FeatureRendering::Hrp);
        println!("Engine: Rendering initialized !");
        Engine { data: self.data, _state: PhantomData }
    }
}
impl Engine<Editor>
{
    fn setup_editor(mut self) -> Engine<Application>
    {
        println!("Engine: editor initialized !");
        Engine { data: self.data, _state: PhantomData }
    }
}

impl Engine<Application>
{
    fn setup_application(mut self) -> Self
    {
        self.data.application_features.push(FeatureApplication::RunInBackground);

        println!("Engine: Application initialized !");
        self
    }

    fn run(self) -> Self
    {
        println!("--------------- ENGINE RUNNING ---------------");
        println!("ENGINE Features: {:?}", self);
        self
    }
}
fn main() { let engine = Engine::<Launch>::new().init().setup_resource().setup_rendering().setup_editor().setup_application().run(); }
