use egui::{Context, Frame};

use crate::engine::{GameState, LoopState, StateData, Trans};
use crate::state::settings::SettingCategory::*;

#[derive(Default)]
pub struct SettingState {
    cur_cat: SettingCategory,
}


#[derive(PartialEq, Eq)]
enum SettingCategory {
    General,
    Video,
    Audio,
}

impl Default for SettingCategory {
    fn default() -> Self {
        General
    }
}

impl GameState for SettingState {
    fn update(&mut self, _: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn render(&mut self, _s: &mut StateData, ctx: &Context) -> Trans {
        egui::SidePanel::left("cats")
            .resizable(false)
            .default_width(128.0)
            .show(ctx, |ui| {
                ui.style_mut().spacing.button_padding *= 8.0;
                ui.vertical_centered(|ui| {
                    ui.selectable_value(&mut self.cur_cat, General, "通常");
                    ui.selectable_value(&mut self.cur_cat, Video, "视频");
                    ui.selectable_value(&mut self.cur_cat, Audio, "音频");
                });
            });
        egui::CentralPanel::default().frame(Frame::none())
            .show(ctx, |_ui| {
                match self.cur_cat {
                    General => {}
                    Video => {}
                    Audio => {}
                }
            });
        Trans::None
    }
}