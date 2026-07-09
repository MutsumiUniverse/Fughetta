use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib, graphene, gsk};
use mutsumi::PlaylistItem;

use super::SourceActionRow;

const ANIM_TAU: f64 = 0.055;
const REMOVE_SLOP: f32 = 24.0;
const DRAG_THRESHOLD: f32 = 6.0;

mod imp {
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    use super::*;

    #[derive(Default)]
    pub struct PlaylistView {
        pub store: RefCell<Option<gio::ListStore>>,
        pub items_changed_id: RefCell<Option<glib::SignalHandlerId>>,

        pub rows: RefCell<Vec<SourceActionRow>>,
        // (top, height)
        pub geometry: RefCell<Vec<(f32, f32)>>,
        // vertical offset
        pub offsets: RefCell<Vec<f32>>,

        pub drag_index: Cell<Option<usize>>,
        pub pending: Cell<Option<usize>>,
        pub gap: Cell<usize>,
        pub drag_dy: Cell<f32>,
        pub remove_pending: Cell<bool>,
        pub releasing: Cell<bool>,
        pub removing: Cell<bool>,
        pub release_alpha: Cell<f32>,

        pub animating: Cell<bool>,
        /// microseconds
        pub last_frame: Cell<i64>,
        pub suppress_rebuild: Cell<bool>,

        /// Currently activated (playing) position in the playlist.
        pub pos: Cell<Option<usize>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistView {
        const NAME: &'static str = "PlaylistView";
        type Type = super::PlaylistView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("list");
        }
    }

    impl ObjectImpl for PlaylistView {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("position-activated")
                        .param_types([i64::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_overflow(gtk::Overflow::Hidden);
            self.setup_reorder_gesture();
        }

        fn dispose(&self) {
            for row in self.rows.borrow_mut().drain(..) {
                row.unparent();
            }
        }
    }

    impl WidgetImpl for PlaylistView {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let rows = self.rows.borrow();

            if orientation == gtk::Orientation::Horizontal {
                let (mut min, mut nat) = (0, 0);
                for row in rows.iter() {
                    let (m, n, _, _) = row.measure(orientation, for_size);
                    min = min.max(m);
                    nat = nat.max(n);
                }
                (min, nat, -1, -1)
            } else {
                let (mut min, mut nat) = (0, 0);
                for row in rows.iter() {
                    let (m, n, _, _) = row.measure(orientation, for_size);
                    min += m;
                    nat += n;
                }
                (min, nat, -1, -1)
            }
        }

        fn size_allocate(&self, width: i32, _height: i32, _baseline: i32) {
            let rows = self.rows.borrow();

            // resting tops/heights
            let mut base = Vec::with_capacity(rows.len());
            let mut y = 0.0f32;
            for row in rows.iter() {
                let (_, nat_h, _, _) = row.measure(gtk::Orientation::Vertical, width);
                let nat_h = nat_h as f32;
                base.push((y, nat_h));
                y += nat_h;
            }
            let total = y;

            // allocate each row at its live position
            // The dragged row follows the pointer; the rest sit at base + offset
            let offsets = self.offsets.borrow();
            let dragged = self.drag_index.get();
            for (i, row) in rows.iter().enumerate() {
                let (top, h) = base[i];
                let dy = if Some(i) == dragged {
                    self.drag_dy.get().clamp(-top, (total - h - top).max(0.0))
                } else {
                    offsets.get(i).copied().unwrap_or(0.0)
                };
                let transform =
                    gsk::Transform::new().translate(&graphene::Point::new(0.0, top + dy));
                row.allocate(width, h as i32, -1, Some(transform));
            }
            drop(offsets);

            *self.geometry.borrow_mut() = base;
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let rows = self.rows.borrow();
            let dragged = self.drag_index.get();

            for (i, row) in rows.iter().enumerate() {
                if Some(i) == dragged {
                    continue;
                }
                self.obj().snapshot_child(row, snapshot);
            }

            if let Some(d) = dragged
                && let Some(row) = rows.get(d)
            {
                let alpha = if self.removing.get() {
                    self.release_alpha.get()
                } else if self.remove_pending.get() {
                    0.35
                } else {
                    1.0
                };

                if alpha < 0.999 {
                    snapshot.push_opacity(alpha as f64);
                    self.obj().snapshot_child(row, snapshot);
                    snapshot.pop();
                } else {
                    self.obj().snapshot_child(row, snapshot);
                }
            }
        }
    }

    impl PlaylistView {
        pub(super) fn set_store(&self, store: Option<gio::ListStore>) {
            if let (Some(old), Some(id)) =
                (self.store.borrow().as_ref(), self.items_changed_id.take())
            {
                old.disconnect(id);
            }

            if let Some(store) = &store {
                let id = store.connect_items_changed(glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_, _, _, _| {
                        if !imp.suppress_rebuild.get() {
                            imp.rebuild();
                        }
                    }
                ));
                self.items_changed_id.replace(Some(id));
            }

            self.store.replace(store);
            self.rebuild();
        }

        fn rebuild(&self) {
            for row in self.rows.borrow_mut().drain(..) {
                row.unparent();
            }

            let mut rows = self.rows.borrow_mut();
            if let Some(store) = self.store.borrow().as_ref() {
                for item in store.iter::<PlaylistItem>().flatten() {
                    let row = SourceActionRow::new(&item);
                    row.set_parent(&*self.obj());
                    self.setup_row(&row);
                    rows.push(row);
                }
            }

            let obj = self.obj();
            if rows.is_empty() {
                obj.remove_css_class("boxed-list-separate");
            } else {
                obj.add_css_class("boxed-list-separate");
            }

            self.offsets.replace(vec![0.0; rows.len()]);
            self.drag_index.set(None);
            self.pending.set(None);
            self.releasing.set(false);
            self.removing.set(false);
            self.remove_pending.set(false);
            self.drag_dy.set(0.0);
            self.release_alpha.set(1.0);
            self.pos.set(None);
            drop(rows);

            if let Some(pos) = self.rows.borrow().iter().position(|r| r.playing()) {
                self.pos.set(Some(pos));
            }

            obj.queue_resize();
        }

        fn setup_row(&self, row: &SourceActionRow) {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[weak]
                row,
                move |_, _, _, _| {
                    if let Some(pos) = imp.row_index(&row) {
                        imp.obj()
                            .emit_by_name::<()>("position-activated", &[&(pos as i64)]);
                    }
                }
            ));
            row.add_controller(click);

            row.connect_delete_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.delete_row(&row)
            ));

            row.connect_move_to_top_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.move_row_to_top(&row)
            ));

            row.connect_move_to_bottom_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.move_row_to_bottom(&row)
            ));

            row.connect_move_up_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.move_row_up(&row)
            ));

            row.connect_move_down_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.move_row_down(&row)
            ));

            row.connect_delete_all_above_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.delete_all_above(&row)
            ));

            row.connect_delete_all_below_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.delete_all_below(&row)
            ));

            row.connect_delete_others_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.delete_all_except(&row)
            ));

            row.connect_delete_all_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_row| imp.delete_all()
            ));

            row.connect_play_next_requested(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |row| imp.play_next(&row)
            ));

            row.connect_played(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[weak]
                row,
                move |_| {
                    if let Some(pos) = imp.row_index(&row) {
                        imp.pos.set(Some(pos));
                    }
                }
            ));
        }

        fn move_row_to_top(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
            {
                store.remove(pos);
                store.insert(0, &item);
            }
        }

        fn move_row_to_bottom(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
            {
                store.remove(pos);
                store.append(&item);
            }
        }

        fn move_row_up(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
                && pos > 0
            {
                store.remove(pos);
                store.insert(pos - 1, &item);
            }
        }

        fn move_row_down(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
                && pos + 1 < store.n_items()
            {
                store.remove(pos);
                store.insert(pos + 1, &item);
            }
        }

        fn delete_row(&self, row: &SourceActionRow) {
            if let Some(item) = row.item() {
                self.remove_item(&item);
            }
        }

        fn delete_all_above(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
            {
                store.splice(0, pos, &[] as &[PlaylistItem]);
            }
        }

        fn delete_all_below(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
            {
                let n = store.n_items();
                if pos + 1 < n {
                    store.splice(pos + 1, n - pos - 1, &[] as &[PlaylistItem]);
                }
            }
        }

        fn delete_all_except(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(&item)
            {
                let n = store.n_items();
                if pos > 0 {
                    store.splice(0, pos, &[] as &[PlaylistItem]);
                }
                if pos + 1 < n {
                    store.splice(1, n - pos - 1, &[] as &[PlaylistItem]);
                }
            }
        }

        fn delete_all(&self) {
            if let Some(store) = self.store.borrow().clone() {
                store.remove_all();
            }
        }

        fn play_next(&self, row: &SourceActionRow) {
            if let Some(item) = row.item()
                && let Some(store) = self.store.borrow().clone()
                && let Some(from) = store.find(&item)
            {
                let n = store.n_items();
                let target = match self.pos.get() {
                    Some(current) => ((current as u32) + 1).min(n),
                    _ => 0,
                };

                if from == target || from + 1 == target {
                    return;
                }

                store.remove(from);
                let insert_at = if from < target { target - 1 } else { target };
                store.insert(insert_at, &item);
            }
        }

        fn setup_reorder_gesture(&self) {
            let drag = gtk::GestureDrag::new();

            drag.connect_drag_begin(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, y| {
                    imp.pending.set(imp.row_at_y(y as f32));
                    imp.drag_index.set(None);
                    imp.drag_dy.set(0.0);
                    imp.remove_pending.set(false);
                    imp.releasing.set(false);
                    imp.removing.set(false);
                    imp.release_alpha.set(1.0);
                }
            ));

            drag.connect_drag_update(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |gesture, ox, oy| {
                    let (ox, oy) = (ox as f32, oy as f32);

                    if imp.drag_index.get().is_none() {
                        if ox.hypot(oy) < DRAG_THRESHOLD {
                            return;
                        }
                        let Some(index) = imp.pending.get() else {
                            return;
                        };
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        imp.drag_index.set(Some(index));
                        imp.gap.set(index);
                        for row in imp.rows.borrow().iter() {
                            row.remove_css_class("dragging");
                        }
                        if let Some(row) = imp.rows.borrow().get(index) {
                            row.add_css_class("dragging");
                        }
                    }

                    let Some((sx, _sy)) = gesture.start_point() else {
                        return;
                    };
                    imp.drag_dy.set(oy);

                    let px = sx as f32 + ox;
                    let width = imp.obj().width() as f32;
                    let remove = px < -REMOVE_SLOP || px > width + REMOVE_SLOP;
                    imp.remove_pending.set(remove);

                    let gap = if remove {
                        imp.gap.get()
                    } else {
                        imp.gap_from_card()
                    };
                    imp.gap.set(gap);
                    imp.start_animation();
                    imp.obj().queue_allocate();
                }
            ));

            drag.connect_drag_end(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _| {
                    imp.pending.set(None);
                    if imp.drag_index.get().is_none() {
                        return;
                    }

                    let clamped = {
                        let geometry = imp.geometry.borrow();
                        imp.clamped_drag_dy(&geometry)
                    };
                    imp.drag_dy.set(clamped);
                    imp.releasing.set(true);
                    imp.removing.set(imp.remove_pending.get());
                    imp.release_alpha
                        .set(if imp.remove_pending.get() { 0.35 } else { 1.0 });
                    imp.start_animation();
                }
            ));

            self.obj().add_controller(drag);
        }

        fn row_index(&self, row: &SourceActionRow) -> Option<usize> {
            self.rows.borrow().iter().position(|r| r == row)
        }

        fn row_at_y(&self, y: f32) -> Option<usize> {
            self.geometry
                .borrow()
                .iter()
                .position(|(base, h)| y >= *base && y < base + h)
        }

        fn remove_item(&self, item: &PlaylistItem) {
            if let Some(store) = self.store.borrow().clone()
                && let Some(pos) = store.find(item)
            {
                store.remove(pos);
            }
        }

        fn clamped_drag_dy(&self, geometry: &[(f32, f32)]) -> f32 {
            let Some(d) = self.drag_index.get() else {
                return 0.0;
            };
            let Some(&(base, h)) = geometry.get(d) else {
                return 0.0;
            };
            let total = geometry.last().map(|(y, hh)| y + hh).unwrap_or(0.0);
            self.drag_dy.get().clamp(-base, (total - h - base).max(0.0))
        }

        fn release_target_dy(&self, geometry: &[(f32, f32)]) -> f32 {
            let Some(d) = self.drag_index.get() else {
                return 0.0;
            };
            let gap = self.gap.get();
            let mut top = 0.0f32;
            let mut seen = 0usize;
            for (j, (_, h)) in geometry.iter().enumerate() {
                if seen == gap {
                    break;
                }
                if j == d {
                    continue;
                }
                top += h;
                seen += 1;
            }
            top - geometry[d].0
        }

        fn gap_from_card(&self) -> usize {
            let geometry = self.geometry.borrow();
            let Some(d) = self.drag_index.get() else {
                return self.gap.get();
            };
            let Some(&(base, _)) = geometry.get(d) else {
                return self.gap.get();
            };
            let card_top = base + self.clamped_drag_dy(&geometry);

            let n = geometry.len();
            let mut best_gap = 0;
            let mut best_dist = f32::MAX;
            let mut rest_top = 0.0f32;
            let mut slot = 0usize;
            for (j, (_, h)) in geometry.iter().enumerate() {
                let dist = (rest_top - card_top).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_gap = slot;
                }
                if j == d {
                    continue;
                }
                rest_top += h;
                slot += 1;
            }
            let dist = (rest_top - card_top).abs();
            if dist < best_dist && n > 0 {
                best_gap = n - 1;
            }
            best_gap
        }

        fn target_offset(&self, i: usize, geometry: &[(f32, f32)]) -> f32 {
            let Some(d) = self.drag_index.get() else {
                return 0.0;
            };
            if i == d {
                return 0.0;
            }

            let gap = self.gap.get();
            let rank = if i < d { i } else { i - 1 };

            let mut target_y = 0.0f32;
            let mut seen = 0usize;
            for (j, (_, h)) in geometry.iter().enumerate() {
                if seen == rank {
                    break;
                }
                if j == d {
                    continue;
                }
                target_y += h;
                seen += 1;
            }
            if rank >= gap && !self.removing.get() {
                target_y += geometry[d].1;
            }

            target_y - geometry[i].0
        }

        pub(super) fn start_animation(&self) {
            if self.animating.replace(true) {
                return;
            }
            self.last_frame.set(0);

            self.obj()
                .add_tick_callback(move |obj, clock| obj.imp().tick(clock));
        }

        fn tick(&self, clock: &gdk::FrameClock) -> glib::ControlFlow {
            let now = clock.frame_time();
            let last = self.last_frame.replace(now);
            let dt = if last == 0 {
                1.0 / 60.0
            } else {
                (now - last) as f64 / 1_000_000.0
            };
            let factor = (1.0 - (-dt / ANIM_TAU).exp()) as f32;

            let geometry = self.geometry.borrow();
            let mut offsets = self.offsets.borrow_mut();
            if offsets.len() != geometry.len() {
                offsets.resize(geometry.len(), 0.0);
            }

            let mut settled = true;
            for i in 0..offsets.len() {
                let target = self.target_offset(i, &geometry);
                let diff = target - offsets[i];
                if diff.abs() < 0.5 {
                    offsets[i] = target;
                } else {
                    offsets[i] += diff * factor;
                    settled = false;
                }
            }

            let mut release_done = false;
            if self.releasing.get() {
                if self.removing.get() {
                    let a = self.release_alpha.get();
                    let na = a + (0.0 - a) * factor;
                    self.release_alpha.set(na);
                    if na > 0.02 {
                        settled = false;
                    }
                } else {
                    let target = self.release_target_dy(&geometry);
                    let dy = self.drag_dy.get();
                    let diff = target - dy;
                    if diff.abs() < 0.5 {
                        self.drag_dy.set(target);
                    } else {
                        self.drag_dy.set(dy + diff * factor);
                        settled = false;
                    }
                }
                release_done = settled;
            }
            drop(offsets);
            drop(geometry);

            self.obj().queue_allocate();

            if release_done {
                self.commit_release();
            }

            if settled && self.drag_index.get().is_none() {
                self.animating.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        }

        fn commit_release(&self) {
            let Some(index) = self.drag_index.get() else {
                return;
            };
            let removing = self.removing.get();
            let gap = self.gap.get();

            if let Some(row) = self.rows.borrow().get(index) {
                row.remove_css_class("dragging");
            }

            self.drag_index.set(None);
            self.releasing.set(false);
            self.removing.set(false);
            self.remove_pending.set(false);
            self.drag_dy.set(0.0);
            self.release_alpha.set(1.0);

            self.suppress_rebuild.set(true);

            if removing {
                if let Some(store) = self.store.borrow().clone()
                    && (index as u32) < store.n_items()
                {
                    store.remove(index as u32);
                }
                let removed = self.rows.borrow_mut().remove(index);
                removed.unparent();
            } else if gap != index {
                if let Some(store) = self.store.borrow().clone()
                    && (index as u32) < store.n_items()
                    && let Some(item) = store.item(index as u32).and_downcast::<PlaylistItem>()
                {
                    store.remove(index as u32);
                    store.insert(gap as u32, &item);
                }
                let mut rows = self.rows.borrow_mut();
                let row = rows.remove(index);
                rows.insert(gap, row);
            }

            self.suppress_rebuild.set(false);

            self.offsets.replace(vec![0.0; self.rows.borrow().len()]);

            let obj = self.obj();
            if self.rows.borrow().is_empty() {
                obj.remove_css_class("boxed-list-separate");
            }
            obj.queue_allocate();
        }
    }
}

glib::wrapper! {
    pub struct PlaylistView(ObjectSubclass<imp::PlaylistView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlaylistView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_store(&self, store: Option<&gio::ListStore>) {
        self.imp().set_store(store.cloned());
    }

    pub fn connect_position_activated<F: Fn(&Self, i64) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_closure(
            "position-activated",
            false,
            glib::closure_local!(move |obj: Self, pos: i64| {
                f(&obj, pos);
            }),
        )
    }
}

impl Default for PlaylistView {
    fn default() -> Self {
        Self::new()
    }
}
