//! Deliberate reference excursions keep one exact writing origin until a successful return.
use super::*;

impl StudioApp {
    pub(super) fn begin_reference_excursion(&mut self, document: Document) -> AppResult<()> {
        self.save_explicit()?;
        let bookmark = if self.reference_return.is_none() {
            Some(navigation_return::NavigationReturn::capture(self)?)
        } else {
            None
        };
        let title = self.document.as_ref().map(|current| current.title.clone());
        self.open_document(document)?;
        if let Some(bookmark) = bookmark {
            self.reference_return = Some(bookmark);
            self.reference_origin_title = title.unwrap_or_default();
        }
        #[cfg(feature = "editor-preview")]
        if self.recovery.is_some() {
            return Ok(());
        }
        self.focus_manuscript = true;
        Ok(())
    }

    pub(super) fn return_to_writing(&mut self) -> AppResult<()> {
        let Some(bookmark) = self.reference_return.take() else {
            return Ok(());
        };
        match bookmark.restore(self) {
            Ok(exact) => {
                self.reference_origin_title.clear();
                self.focus_manuscript = true;
                if !exact {
                    self.status = "The original document changed. Its latest text is open.".into();
                }
                Ok(())
            }
            Err(error) => {
                self.reference_return = Some(bookmark);
                Err(error)
            }
        }
    }

    pub(super) fn reference_return_control(&mut self, ui: &mut egui::Ui) {
        if self.reference_return.is_none() {
            return;
        }
        let label = self.language.format(
            "Return to {title}",
            &[("title", self.reference_origin_title.clone())],
        );
        if ui
            .add_sized(
                [ui.available_width(), 40.0],
                egui::Button::new(&label).truncate(),
            )
            .on_hover_text(label)
            .clicked()
        {
            let result = self.return_to_writing();
            self.record_error(result);
        }
        ui.add_space(8.0);
    }
}
