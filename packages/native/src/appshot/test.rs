use super::{
    one_shot_png, AppshotPermission, AppshotPermissionStatus, AppshotSelection, AppshotState,
};

impl AppshotState {
    pub(crate) fn set_test_selection(&mut self, selected: bool) {
        self.test_selection = Some(selected);
    }

    pub(crate) fn select_test_window(&mut self) -> AppshotSelection {
        match self.test_selection.take().unwrap_or(false) {
            true => self.issue_selected_handle(),
            false => Self::cancelled(),
        }
    }

    pub(crate) fn set_test_permission(&mut self, granted: bool) {
        self.test_permission = granted;
    }

    pub(crate) fn test_permission(&self) -> AppshotPermission {
        AppshotPermission {
            status: if self.test_permission {
                AppshotPermissionStatus::Granted
            } else {
                AppshotPermissionStatus::Missing
            },
            restart_required: false,
        }
    }

    pub(crate) fn capture_test_handle(
        &mut self,
        handle: &str,
    ) -> napi::Result<napi::bindgen_prelude::Buffer> {
        self.consume_handle(handle)?;
        Ok(one_shot_png())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appshot::AppshotSelectionStatus;

    #[test]
    fn selected_handle_is_single_use_and_opaque() {
        let mut state = AppshotState::default();
        state.set_test_selection(true);
        let selection = state.select_test_window();
        assert_eq!(selection.status, AppshotSelectionStatus::Selected);
        let handle = selection.handle.expect("opaque handle");
        assert!(handle.starts_with("appshot-"));
        let png = state.capture_test_handle(&handle).unwrap();
        let bytes = png.as_ref();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let mut offset = 8;
        let mut chunks = Vec::new();
        while offset < bytes.len() {
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            let name = &bytes[offset + 4..offset + 8];
            chunks.push(name);
            offset += 12 + length;
        }
        assert_eq!(offset, bytes.len());
        assert_eq!(chunks.first(), Some(&b"IHDR".as_slice()));
        assert!(chunks.contains(&b"IDAT".as_slice()));
        assert_eq!(chunks.last(), Some(&b"IEND".as_slice()));
        assert!(state.capture_test_handle(&handle).is_err());
    }

    #[test]
    fn cancellation_and_permission_are_metadata_free() {
        let mut state = AppshotState::default();
        assert_eq!(
            state.select_test_window(),
            AppshotSelection {
                status: AppshotSelectionStatus::Cancelled,
                handle: None
            }
        );
        state.set_test_permission(false);
        assert_eq!(
            state.test_permission(),
            AppshotPermission {
                status: AppshotPermissionStatus::Missing,
                restart_required: false
            }
        );
    }
}
