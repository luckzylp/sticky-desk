#[derive(Clone, Debug)]
pub enum TrayCommand {
    NewNote,
    NewNoteFromClipboard,
    ShowAllNotes,
    HideAllNotes,
    Notebook(String),
    NewNotebook,
    Explorer,
    Settings,
    About,
    Exit,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::TrayCommand;
    use ksni::menu::{MenuItem, StandardItem, SubMenu};
    use ksni::{Icon, ToolTip, Tray, TrayMethods};
    use std::sync::{Arc, Mutex, mpsc::Sender};

    #[derive(Clone)]
    pub struct SsnTray {
        pub sender: Sender<TrayCommand>,
        pub notebooks: Arc<Mutex<Vec<String>>>,
    }

    fn item(label: &str, cmd: TrayCommand, sender: &Sender<TrayCommand>) -> MenuItem<SsnTray> {
        let sender = sender.clone();
        StandardItem {
            label: label.to_string(),
            activate: Box::new(move |_this: &mut SsnTray| {
                let _ = sender.send(cmd.clone());
            }),
            ..Default::default()
        }
        .into()
    }

    fn icon() -> Vec<Icon> {
        let bytes = include_bytes!("../../assets/images/sticky32.png");
        match image::load_from_memory(bytes) {
            Ok(img) => {
                let img = img.to_rgba8();
                let (w, h) = img.dimensions();
                let mut data = Vec::with_capacity((w * h * 4) as usize);
                for px in img.pixels() {
                    data.extend_from_slice(&[px[3], px[0], px[1], px[2]]);
                }
                vec![Icon {
                    width: w as i32,
                    height: h as i32,
                    data,
                }]
            }
            Err(_) => Vec::new(),
        }
    }

    impl Tray for SsnTray {
        fn id(&self) -> String {
            "sticky-desk".into()
        }

        fn title(&self) -> String {
            "StickyDesk".into()
        }

        fn tool_tip(&self) -> ToolTip {
            ToolTip {
                title: "StickyDesk".into(),
                description: "Double click to create a new note".into(),
                ..Default::default()
            }
        }

        fn icon_pixmap(&self) -> Vec<Icon> {
            icon()
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            let _ = self.sender.send(TrayCommand::ShowAllNotes);
        }

        fn secondary_activate(&mut self, _x: i32, _y: i32) {
            let _ = self.sender.send(TrayCommand::NewNote);
        }

        fn menu(&self) -> Vec<MenuItem<Self>> {
            let s = &self.sender;
            let notebooks: Vec<String> =
                self.notebooks.lock().map(|n| n.clone()).unwrap_or_default();
            let mut nb_menu = SubMenu {
                label: "Notebooks".into(),
                ..Default::default()
            };
            for nb in notebooks {
                let sender = s.clone();
                let label = nb.clone();
                nb_menu.submenu.push(MenuItem::Standard(StandardItem {
                    label: nb.clone(),
                    activate: Box::new(move |_this: &mut SsnTray| {
                        let _ = sender.send(TrayCommand::Notebook(label.clone()));
                    }),
                    ..Default::default()
                }));
            }
            nb_menu.submenu.push(MenuItem::Separator);
            nb_menu
                .submenu
                .push(item("New Notebook", TrayCommand::NewNotebook, s));
            vec![
                item("New Note", TrayCommand::NewNote, s),
                item(
                    "New Note from clipboard",
                    TrayCommand::NewNoteFromClipboard,
                    s,
                ),
                MenuItem::Separator,
                item("Show All Notes", TrayCommand::ShowAllNotes, s),
                item("Hide All Notes", TrayCommand::HideAllNotes, s),
                MenuItem::Separator,
                MenuItem::SubMenu(nb_menu),
                MenuItem::Separator,
                item("Explorer", TrayCommand::Explorer, s),
                item("Settings", TrayCommand::Settings, s),
                item("About", TrayCommand::About, s),
                MenuItem::Separator,
                item("Exit", TrayCommand::Exit, s),
            ]
        }
    }

    pub fn start(sender: Sender<TrayCommand>, notebooks: Vec<String>) {
        let notebooks = Arc::new(Mutex::new(notebooks));
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };
            rt.block_on(async move {
                let mut logged = false;
                loop {
                    let tray = SsnTray {
                        sender: sender.clone(),
                        notebooks: notebooks.clone(),
                    };
                    match tray.spawn().await {
                        Ok(_handle) => {
                            eprintln!("tray: registered StatusNotifierItem");
                            tokio::time::sleep(std::time::Duration::from_secs(u64::MAX / 2)).await;
                        }
                        Err(e) => {
                            if !logged {
                                eprintln!("tray: waiting for StatusNotifierWatcher: {e}");
                                logged = true;
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        });
    }
}

#[cfg(target_os = "linux")]
pub use linux::start;

#[cfg(not(target_os = "linux"))]
pub fn start(_sender: Sender<TrayCommand>, _notebooks: Vec<String>) {}
