use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};
use mutsumi::PlaylistItem;

use crate::playlist::PlaylistView;

/// Seconds for the ripple to sweep across the row once.
const SWEEP_PERIOD: f64 = 0.9;
/// Nominal spacing between grain particles, in pixels.
const GRAIN_SPACING: f32 = 9.0;
/// How far ahead of the wave front particles start to glow (px).
const LEAD: f32 = 26.0;
/// How far the comet tail trails behind the wave front (px).
const TAIL: f32 = 120.0;

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};
    

    use glib::subclass::InitializingObject;
    use gtk::PopoverMenu;
    use gtk::gdk::Rectangle;
    use gtk::glib::WeakRef;

    use crate::playlist::PlaylistView;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/mutsumiuniverse/fughetta/ui/source_row.ui")]
    #[properties(wrapper_type = super::SourceActionRow)]
    pub struct SourceActionRow {
        #[property(get, set = Self::set_item, nullable)]
        pub item: RefCell<Option<PlaylistItem>>,

        pub bindings: RefCell<Vec<glib::Binding>>,

        #[property(get, set = Self::set_playing)]
        pub playing: Cell<bool>,

        pub anim_start: Cell<i64>,
        pub tick_id: RefCell<Option<gtk::TickCallbackId>>,

        pub right_menu: OnceCell<PopoverMenu>,

        pub view: WeakRef<PlaylistView>,
    }

    impl SourceActionRow {
        fn set_item(&self, item: Option<PlaylistItem>) {
            for binding in self.bindings.borrow_mut().drain(..) {
                binding.unbind();
            }

            if let Some(item) = &item {
                let obj = self.obj();
                let mut bindings = self.bindings.borrow_mut();

                bindings.push(
                    item.bind_property("filename", &*obj, "title")
                        .transform_to(|_, filename: String| {
                            Some(mutsumi::title_from_uri(&filename))
                        })
                        .sync_create()
                        .build(),
                );

                bindings.push(
                    item.bind_property("filename", &*obj, "subtitle")
                        .sync_create()
                        .build(),
                );

                bindings.push(
                    item.bind_property("current", &*obj, "playing")
                        .sync_create()
                        .build(),
                );
            }

            self.item.replace(item);
        }

        fn set_playing(&self, playing: bool) {
            if self.playing.get() == playing {
                return;
            }
            self.playing.set(playing);

            let obj = self.obj();
            if playing {
                obj.add_css_class("playing");

                if let Some(clock) = obj.frame_clock() {
                    self.anim_start.set(clock.frame_time());
                }
                if let Some(id) = self.tick_id.take() {
                    id.remove();
                }
                let id = obj.add_tick_callback(|obj, clock| {
                    let imp = obj.imp();
                    let elapsed = (clock.frame_time() - imp.anim_start.get()) as f64 / 1_000_000.0;
                    obj.queue_draw();
                    if elapsed >= SWEEP_PERIOD {
                        imp.tick_id.replace(None);
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
                self.tick_id.replace(Some(id));

                obj.set_view_playing_position();
            } else {
                obj.remove_css_class("playing");
                if let Some(id) = self.tick_id.take() {
                    id.remove();
                }
                obj.queue_draw();
            }
        }

        fn elapsed(&self) -> f64 {
            let obj = self.obj();
            let Some(clock) = obj.frame_clock() else {
                return 0.0;
            };
            (clock.frame_time() - self.anim_start.get()) as f64 / 1_000_000.0
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SourceActionRow {
        const NAME: &'static str = "SourceActionRow";
        type Type = super::SourceActionRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("row.remove", None, |obj, _, _| {
                obj.remove_row();
            });

            klass.install_action("row.move-top", None, |obj, _, _| {
                obj.move_to_top();
            });

            klass.install_action("row.move-bottom", None, |obj, _, _| {
                obj.move_to_bottom();
            });

            klass.install_action("row.move-up", None, |obj, _, _| {
                obj.move_up();
            });

            klass.install_action("row.move-down", None, |obj, _, _| {
                obj.move_down();
            });

            klass.install_action("row.remove-above", None, |obj, _, _| {
                obj.remove_above();
            });

            klass.install_action("row.remove-below", None, |obj, _, _| {
                obj.remove_below();
            });

            klass.install_action("row.remove-others", None, |obj, _, _| {
                obj.remove_others();
            });

            klass.install_action("row.remove-all", None, |obj, _, _| {
                obj.remove_all();
            });

            klass.install_action("row.play-next", None, |obj, _, _| {
                obj.play_next();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SourceActionRow {
        fn constructed(&self) {
            self.parent_constructed();

            self.set_right_menu();
        }

        fn dispose(&self) {
            if let Some(right_menu) = self.right_menu.get() {
                right_menu.unparent();
            }

            if let Some(id) = self.tick_id.take() {
                id.remove();
            }
        }
    }

    impl WidgetImpl for SourceActionRow {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let playing = self.playing.get();
            let obj = self.obj();
            let w = obj.width() as f32;
            let h = obj.height() as f32;

            if !playing || w <= 0.0 || h <= 0.0 {
                self.parent_snapshot(snapshot);
                return;
            }

            let elapsed = self.elapsed();
            let color = obj.color();

            super::draw_wash(snapshot, w, h, color, elapsed);

            self.parent_snapshot(snapshot);

            if elapsed < SWEEP_PERIOD {
                super::draw_grain(snapshot, w, h, color, elapsed);
            }
        }
    }

    impl ListBoxRowImpl for SourceActionRow {}
    impl PreferencesRowImpl for SourceActionRow {}
    impl ActionRowImpl for SourceActionRow {}

    #[gtk::template_callbacks]
    impl SourceActionRow {
        fn set_right_menu(&self) {
            let builder =
                gtk::Builder::from_resource("/io/github/mutsumiuniverse/fughetta/ui/row_menu.ui");

            let menu_model = builder
                .object::<gtk::gio::Menu>("row-menu")
                .expect("row-menu should exist in row_menu.ui");

            let popover = PopoverMenu::builder()
                .menu_model(&menu_model)
                .halign(gtk::Align::Start)
                .has_arrow(false)
                .build();

            popover.set_parent(&*self.obj());
            self.right_menu
                .set(popover)
                .expect("right_menu OnceCell should only be set once");
        }

        #[template_callback]
        fn on_delete_clicked(&self) {
            self.obj().emit_by_name::<()>("delete-requested", &[]);
        }

        #[template_callback]
        fn right_click_cb(&self, _n: i32, x: f64, y: f64) {
            let Some(right_menu) = self.right_menu.get() else {
                return;
            };

            right_menu.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
            right_menu.popup();
        }

        #[template_callback]
        fn left_click_cb(&self, _n: i32, _x: f64, _y: f64) {
            let Some(view) = self.view.upgrade() else {
                return;
            };

            let Some(pos) = view.imp().row_index(&self.obj()) else {
                return;
            };

            view.emit_by_name::<()>("position-activated", &[&(pos as i64)]);
        }
    }
}

glib::wrapper! {
    pub struct SourceActionRow(ObjectSubclass<imp::SourceActionRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl SourceActionRow {
    pub fn new(item: &PlaylistItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }

    pub fn set_view(&self, view: Option<&PlaylistView>) {
        self.imp().view.set(view);
    }

    pub fn set_view_playing_position(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().set_playing_position(self);
    }

    pub fn move_to_top(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().move_row_to_top(self);
    }

    pub fn move_to_bottom(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().move_row_to_bottom(self);
    }

    pub fn move_up(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().move_row_up(self);
    }

    pub fn move_down(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().move_row_down(self);
    }

    pub fn remove_row(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().remove_row(self);
    }

    pub fn remove_above(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().remove_all_above(self);
    }

    pub fn remove_below(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().remove_all_below(self);
    }

    pub fn remove_others(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().remove_all_except(self);
    }

    pub fn remove_all(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().remove_all_rows();
    }

    pub fn play_next(&self) {
        let Some(view) = self.imp().view.upgrade() else {
            return;
        };

        view.imp().play_next(self);
    }
}

impl Default for SourceActionRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

// Small deterministic hash → [0,1), so each particle has stable jitter/flicker
// without allocating any state.
fn hash01(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Paint the accent wash, revealed left→right up to the wave front. Once the
/// sweep finishes (`t >= SWEEP_PERIOD`) it covers the whole row. Drawn *behind*
/// the row content. `t` is elapsed seconds; `color` is the accent color.
fn draw_wash(snapshot: &gtk::Snapshot, w: f32, h: f32, color: gtk::gdk::RGBA, t: f64) {
    use gtk::{graphene, gsk};

    let bounds = graphene::Rect::new(0.0, 0.0, w, h);
    let clip = gsk::RoundedRect::from_rect(bounds, 12.0);
    snapshot.push_rounded_clip(&clip);

    let cr = snapshot.append_cairo(&bounds);
    let (r, g, b) = (
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
    );

    let phase = (t / SWEEP_PERIOD).clamp(0.0, 1.0) as f32;

    if phase >= 1.0 {
        // Sweep done: flat accent wash across the whole row.
        cr.set_source_rgba(r, g, b, 0.14);
        cr.rectangle(0.0, 0.0, w as f64, h as f64);
        let _ = cr.fill();
        drop(cr);
        snapshot.pop();
        return;
    }

    // Reveal edge travels 0 → w across the sweep, with a soft feathered front.
    let edge = phase * w;
    let feather = LEAD.min(edge.max(1.0));

    let grad = gtk::cairo::LinearGradient::new(0.0, 0.0, (edge + feather) as f64, 0.0);
    let split = if edge + feather > 0.0 {
        ((edge - feather) / (edge + feather)).clamp(0.0, 1.0) as f64
    } else {
        0.0
    };
    grad.add_color_stop_rgba(0.0, r, g, b, 0.14);
    grad.add_color_stop_rgba(split, r, g, b, 0.14);
    grad.add_color_stop_rgba(1.0, r, g, b, 0.0);
    let _ = cr.set_source(&grad);
    cr.rectangle(0.0, 0.0, (edge + feather) as f64, h as f64);
    let _ = cr.fill();

    drop(cr);
    snapshot.pop();
}

/// Paint the grainy comet that rides the leading edge of the reveal. Drawn
/// *over* the row content so the sparkle reads on top. `t` is elapsed seconds.
fn draw_grain(snapshot: &gtk::Snapshot, w: f32, h: f32, color: gtk::gdk::RGBA, t: f64) {
    use gtk::{graphene, gsk};

    let bounds = graphene::Rect::new(0.0, 0.0, w, h);
    let clip = gsk::RoundedRect::from_rect(bounds, 12.0);
    snapshot.push_rounded_clip(&clip);

    let cr = snapshot.append_cairo(&bounds);
    let (cr_r, cr_g, cr_b) = (
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
    );

    // Wave front rides exactly at the wash reveal edge (0 → w), so the grain
    // tracks the colour instead of trailing behind it.
    let phase = (t / SWEEP_PERIOD).clamp(0.0, 1.0) as f32;
    let front = phase * w;

    // Grain particles on a jittered grid.
    let cols = (w / GRAIN_SPACING).ceil() as u32 + 1;
    let rows = (h / GRAIN_SPACING).ceil() as u32 + 1;

    for gy in 0..rows {
        for gx in 0..cols {
            let jx = hash01(gx, gy);
            let jy = hash01(gx.wrapping_add(101), gy.wrapping_add(53));
            let px = (gx as f32 + jx) * GRAIN_SPACING;
            let py = (gy as f32 + jy) * GRAIN_SPACING;
            if px > w || py > h {
                continue;
            }

            // Distance from the wave front (positive = behind the front).
            let d = front - px;

            // Asymmetric comet falloff: a sharp lead edge, a long trailing tail.
            let intensity = if d < 0.0 {
                let a = 1.0 + d / LEAD; // d in [-LEAD, 0] → [0,1]
                if a <= 0.0 {
                    continue;
                }
                a * a
            } else {
                let a = 1.0 - d / TAIL;
                if a <= 0.0 {
                    continue;
                }
                a * a * a
            };

            // Per-particle twinkle so the grain shimmers as the wave passes.
            let seed = hash01(gx.wrapping_add(7), gy.wrapping_add(211));
            let flick =
                0.55 + 0.45 * ((t as f32 * 9.0 + seed * std::f32::consts::TAU + px * 0.05).sin());

            let radius = (0.6 + 1.7 * hash01(gx.wrapping_add(313), gy)) as f64;
            let alpha = (intensity * flick).clamp(0.0, 1.0) as f64 * 0.9;
            if alpha < 0.02 {
                continue;
            }

            cr.set_source_rgba(cr_r, cr_g, cr_b, alpha);
            cr.arc(px as f64, py as f64, radius, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }
    }

    drop(cr);
    snapshot.pop();
}
