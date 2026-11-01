//! Semantic state at the public input boundary. No drawing or input dispatch.
use super::*;
impl Menu {
    pub fn selection_state(&self, av: &AvConfig, cfg: &InputConfig) -> Vec<(String, String)> {
        let context=vec![("atlas.domain".into(),format!("{:?}",self.atlas.domain)),("atlas.section".into(),self.atlas.selected_section().to_string()),("atlas.sidebar_focus".into(),self.atlas.sidebar_focus.to_string())];
        if let Some(index)=self.input_tabs_focus {
            let mut state=context;state.extend([("page".into(),format!("{:?}",self.page)),("item_id".into(),format!("input-tab/{index}")),("label".into(),["Tastiera","Gamepad","Mouse e tastiera"][index as usize].into()),("input_device".into(),format!("{:?}",cfg.device))]);return state;
        }
        if self.tool_active() {let mut state=self.tool.selection_state();state.extend(context);return state;}
        if self.content_active() {let mut state=self.content.selection_state();state.extend(context);return state;}
        if self.cheat_active() {let mut state=self.cheats.selection_state();state.extend(context);return state;}
        if self.page==Page::Information{let mut state=context;state.extend([("page".into(),"Information".into()),("readonly".into(),"true".into()),("detail_scroll".into(),self.atlas.detail_scroll.to_string())]);return state;}
        if self.atlas_reference_active(){let items=self.atlas_reference_items();let cursor=self.atlas_reference_cursor();let mut state=context;state.extend([("page".into(),if self.atlas.planned {"DebugV2"}else{"CliReference"}.into()),("row".into(),cursor.to_string()),("item_id".into(),items.get(cursor).map(|v|v.0.clone()).unwrap_or_default()),("readonly".into(),"true".into()),("detail_scroll".into(),self.atlas.detail_scroll.to_string())]);return state;}
        let mut state = vec![("page".into(), format!("{:?}", self.page)), ("row".into(), self.selected.to_string())];
        if let Some(row) = self.rows().get(self.selected).copied() {
            state.push(("item_id".into(), format!("{:?}/{row:?}", self.page)));
            state.push(("label".into(), row.localized_label(av.menu_language).into()));
            let pad = self.page == Page::Gamepad;
            let keys = if pad { &cfg.pads } else if self.page == Page::MouseKeyboard { &cfg.mouse_keys } else { &cfg.keys };
            let value = match row {
                Row::FastForwardToggle => Some(self.atlas.fast_active.to_string()),
                Row::Menu => Some(if pad { &cfg.menu_pad } else { &cfg.menu_key }.clone()),
                Row::FfKey => Some(if pad { &cfg.ff_pad } else { &cfg.ff_key }.clone()),
                Row::FfToggleKey => Some(if pad { &cfg.ff_toggle_pad } else { &cfg.ff_toggle_key }.clone()),
                Row::ZoomIn => Some(if pad { &cfg.zoom_in_pad } else { &cfg.zoom_in_key }.clone()),
                Row::ZoomOut => Some(if pad { &cfg.zoom_out_pad } else { &cfg.zoom_out_key }.clone()),
                Row::Key(button) => Some(keys[button.index()].clone()),
                Row::MouseMode => Some(cfg.mouse_keyboard.to_string()),
                Row::ToolInput(slot) => Some(self.tool_inputs.iter().find(|(key,_)| *key == slot).map(|(_,v)|v.clone()).unwrap_or_default()),
                _ => value_of(row, av, keys, &cfg.ff_key, self.capture),
            };
            if let Some(value) = value { state.push(("value".into(), value)); }
        }
        state.push(("atlas.domain".into(),format!("{:?}",self.atlas.domain)));
        state.push(("atlas.section".into(),self.atlas.selected_section().to_string()));
        state.push(("atlas.sidebar_focus".into(),self.atlas.sidebar_focus.to_string()));
        state.push(("capture".into(), self.capture.map(|b|format!("{b:?}")).unwrap_or_default()));
        state.push(("input_device".into(), format!("{:?}",cfg.device)));
        let feedback=match self.page {
            Page::Main=>&self.session_message,
            Page::SaveStates|Page::ReplaceState=>&self.state_message,
            Page::GameSaves|Page::SwitchGameSave|Page::ReplaceGameSave=>&self.game_save_message,
            Page::SaveBackups|Page::RestoreBackup=>&self.backup_message,
            _=>&self.debug_message,
        };
        state.push(("feedback".into(), feedback.clone()));
        state.push(("state_slot".into(), self.state_slot.to_string()));
        state.push(("game_save_slot".into(), self.game_save_slot.to_string()));
        if matches!(self.page,Page::SaveStates|Page::ReplaceState){
            if let Some(slot)=self.state_slots.get(self.state_slot){state.push(("state_slot.status".into(),slot.status.clone()));state.push(("state_slot.occupied".into(),slot.occupied.to_string()));}
            state.push(("state_undo".into(),self.state_undo.clone().unwrap_or_default()));
        }
        if matches!(self.page,Page::GameSaves|Page::SwitchGameSave|Page::ReplaceGameSave){
            if let Some(slot)=self.game_save_slots.get(self.game_save_slot){state.extend([("game_save.name".into(),slot.name.clone()),("game_save.status".into(),slot.status.clone()),("game_save.available".into(),slot.available.to_string()),("game_save.current".into(),slot.current.to_string()),("game_save.occupied".into(),slot.occupied.to_string())]);}
        }
        if matches!(self.page,Page::SaveBackups|Page::RestoreBackup){
            state.push(("backup_selection".into(),format!("{:?}",self.backup_selection)));
            for(index,slot)in self.save_backups.iter().enumerate(){state.push((format!("backup.{index}.generation"),format!("{:?}",slot.generation)));state.push((format!("backup.{index}.status"),slot.status.clone()));}
        }
        if let Some(dropdown) = &self.dropdown {
            state.push(("dropdown.id".into(), format!("{:?}",dropdown.row)));
            state.push(("dropdown.index".into(), dropdown.index.to_string()));
            state.push(("dropdown.value".into(), dropdown.options.get(dropdown.index).cloned().unwrap_or_default()));
        }
        if self.page == Page::DebugCatalog { state.extend(self.catalog.selection_state()); }
        state
    }
}

#[cfg(test)] mod tests;
