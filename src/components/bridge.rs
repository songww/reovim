#[derive(Debug)]
pub struct VimBridge {}

impl relm4::Component for VimBridge {
    type Init = crate::Opts;
    type Root = ();
    type Input = ();
    type Output = ();
    type Widgets = ();
    type CommandOutput = ();

    fn init(
        opts: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        //
        sender.command(|out, shutdown| {
            shutdown
                .register(async move { crate::bridge::open(opts).await })
                .drop_on_shutdown()
        });
        relm4::ComponentParts {
            model: VimBridge {},
            widgets: (),
        }
    }

    fn init_root() -> Self::Root {
        ()
    }

    fn update(
        &mut self,
        message: Self::Input,
        sender: relm4::ComponentSender<Self>,
        root: &Self::Root,
    ) {
        //
    }
}
