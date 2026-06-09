use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardAction {
    Copy,
    Cut,
}

#[derive(Clone, Copy)]
pub struct ClipboardState {
    files: ReadSignal<Vec<String>>,
    set_files: WriteSignal<Vec<String>>,
    action: ReadSignal<Option<ClipboardAction>>,
    set_action: WriteSignal<Option<ClipboardAction>>,
}

impl ClipboardState {
    pub fn files(&self) -> Vec<String> {
        self.files.get()
    }

    pub fn action(&self) -> Option<ClipboardAction> {
        self.action.get()
    }

    pub fn has_files(&self) -> bool {
        !self.files.with(Vec::is_empty)
    }

    pub fn file_count(&self) -> usize {
        self.files.with(Vec::len)
    }

    pub fn copy_files(&self, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        self.set_files.set(paths);
        self.set_action.set(Some(ClipboardAction::Copy));
    }

    pub fn cut_files(&self, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        self.set_files.set(paths);
        self.set_action.set(Some(ClipboardAction::Cut));
    }

    pub fn clear(&self) {
        self.set_files.set(Vec::new());
        self.set_action.set(None);
    }
}

pub fn provide_clipboard_state() -> ClipboardState {
    let (files, set_files) = create_signal(Vec::<String>::new());
    let (action, set_action) = create_signal(None::<ClipboardAction>);

    let state = ClipboardState {
        files,
        set_files,
        action,
        set_action,
    };

    provide_context(state);
    state
}

pub fn use_clipboard_state() -> ClipboardState {
    use_context::<ClipboardState>().expect("ClipboardState not provided")
}
