mod scene;
use eframe::egui;
use scene::{Brick, Event, NoopServices, SceneDefinition, SceneManager, SceneTransition, Sprite};
use std::collections::HashMap;

struct PocketCat {
    manager: SceneManager,
    selected: String,
    event: Event,
    new_scene: String,
    variable_name: String,
    variable_value: String,
    status: String,
    running: bool,
    last_tick: std::time::Instant,
}
impl Default for PocketCat {
    fn default() -> Self {
        let make_sprite = |id: &str, x, y| Sprite { id: id.into(), x, y, costume: 0, visible: true, is_clone: false, local_variables: HashMap::new() };
        let defs = vec![
            SceneDefinition { id: "menu".into(), name: "Меню".into(), sprites: vec![make_sprite("Котик", 0.0, 0.0)], scripts: vec![scene::Script { event: Event::Start, bricks: vec![Brick::Say("Нажми Играть!".into())] }] },
            SceneDefinition { id: "level1".into(), name: "Уровень 1".into(), sprites: vec![make_sprite("Котик", -60.0, 0.0), make_sprite("Звезда", 70.0, 30.0)], scripts: vec![scene::Script { event: Event::Start, bricks: vec![Brick::SetVariable { name: "Очки".into(), value: 0.0 }] }] },
        ];
        let mut manager = SceneManager::new(defs);
        manager.transition("menu", SceneTransition::StartScene, &mut NoopServices).unwrap();
        Self { manager, selected: "menu".into(), event: Event::Start, new_scene: String::new(), variable_name: "Очки".into(), variable_value: "0".into(), status: "Сцена загружена · добавляй кирпичики".into(), running: false, last_tick: std::time::Instant::now() }
    }
}
impl PocketCat {
    fn insert(&mut self, brick: Brick) {
        match self.manager.add_brick(&self.selected, self.event.clone(), brick) {
            Ok(()) => self.status = "Кирпичик добавлен в сценарий".into(), Err(e) => self.status = e,
        }
    }
    fn current_def(&self) -> Option<&SceneDefinition> { self.manager.scene_definitions().find(|d| d.id == self.selected) }
}
impl eframe::App for PocketCat {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        if self.running {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.last_tick).as_secs_f64(); self.last_tick = now;
            self.manager.tick(dt.min(0.1)); ctx.request_repaint();
        } else { self.last_tick = std::time::Instant::now(); }
        egui::TopBottomPanel::top("header").exact_height(56.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(egui::RichText::new("🐾 POCKETCAT").size(19.0).strong().color(egui::Color32::from_rgb(190, 160, 255)));
                ui.separator(); ui.label("Визуальный конструктор игр");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.running { "■ Стоп" } else { "▶ Играть" }).clicked() { self.running = !self.running; self.status = if self.running { "Игра выполняется" } else { "Игра остановлена" }.into(); }
                    if ui.button("⟲ Сброс сцены").clicked() { self.manager.reset_scene_cache(&self.selected, &mut NoopServices); let _ = self.manager.transition(&self.selected, SceneTransition::StartScene, &mut NoopServices); self.status = "Сцена сброшена в исходное состояние".into(); }
                });
            });
        });
        egui::SidePanel::left("scenes").resizable(false).default_width(178.0).show(ctx, |ui| {
            ui.add_space(12.0); ui.label(egui::RichText::new("МОИ СЦЕНЫ").strong().size(12.0).color(egui::Color32::GRAY)); ui.add_space(6.0);
            let scenes: Vec<(String,String)> = self.manager.scene_definitions().map(|d|(d.id.clone(),d.name.clone())).collect();
            for (id,name) in scenes {
                if ui.selectable_label(id == self.selected, format!("▣  {name}")).clicked() {
                    let mode = SceneTransition::ResumeScene;
                    match self.manager.transition(&id, mode, &mut NoopServices) { Ok(()) => { self.selected = id; self.status = "Сцена продолжена из памяти".into(); }, Err(e) => self.status = e }
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| { ui.text_edit_singleline(&mut self.new_scene); if ui.small_button("＋").clicked() && !self.new_scene.trim().is_empty() {
                let id = format!("scene{}", self.manager.scene_definitions().count()+1);
                let def = SceneDefinition { id: id.clone(), name: self.new_scene.trim().to_string(), sprites: vec![Sprite { id: "Котик".into(), x:0.0,y:0.0,costume:0,visible:true,is_clone:false,local_variables:HashMap::new() }], scripts: vec![scene::Script { event: Event::Start, bricks: vec![] }] };
                if self.manager.add_scene(def).is_ok() { self.new_scene.clear(); let _ = self.manager.transition(&id, SceneTransition::StartScene, &mut NoopServices); self.selected=id; self.status="Новая сцена создана".into(); }
            }});
            ui.add_space(12.0); ui.label(egui::RichText::new("ПЕРЕХОД").strong().size(12.0).color(egui::Color32::GRAY));
            ui.label(egui::RichText::new("Выбери сцену, чтобы продолжить её из памяти. «Сброс сцены» начинает заново.").size(11.0).weak());
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| { ui.label(egui::RichText::new("Rust · без VM и GC").size(11.0).weak()); });
        });
        egui::SidePanel::right("bricks").resizable(false).default_width(260.0).show(ctx, |ui| {
            ui.add_space(12.0); ui.label(egui::RichText::new("КИРПИЧИКИ").strong().size(12.0).color(egui::Color32::GRAY));
            egui::ComboBox::from_label("Событие").selected_text(match self.event { Event::Start=>"Когда при старте", Event::Resume=>"Когда возобновлена", Event::Touch=>"Когда касание", Event::Always=>"Всегда" }).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.event, Event::Start, "Когда при старте"); ui.selectable_value(&mut self.event, Event::Resume, "Когда возобновлена"); ui.selectable_value(&mut self.event, Event::Touch, "Когда касание"); ui.selectable_value(&mut self.event, Event::Always, "Всегда");
            });
            ui.add_space(7.0); ui.label(egui::RichText::new("СОБЫТИЯ").color(egui::Color32::from_rgb(240,180,80)).strong());
            for e in [(Event::Start,"⚑ При старте"),(Event::Resume,"↻ При возобновлении"),(Event::Touch,"☝ При касании"),(Event::Always,"∞ Всегда")] { if ui.button(e.1).clicked() { self.event = e.0; } }
            ui.add_space(8.0); ui.label(egui::RichText::new("ДВИЖЕНИЕ И ФИЗИКА").color(egui::Color32::from_rgb(90,175,245)).strong());
            if ui.button("↗ Изменить X на 10").clicked(){self.insert(Brick::ChangeX(10.0));}
            if ui.button("↕ Изменить Y на 10").clicked(){self.insert(Brick::ChangeY(10.0));}
            if ui.button("⬇ Гравитация −9.8").clicked(){self.insert(Brick::SetGravity(-9.8));}
            if ui.button("⤴ Отскок").clicked(){self.insert(Brick::Bounce);}
            ui.add_space(8.0); ui.label(egui::RichText::new("ПЕРЕМЕННЫЕ").color(egui::Color32::from_rgb(190,140,250)).strong());
            ui.horizontal(|ui|{ui.text_edit_singleline(&mut self.variable_name); ui.text_edit_singleline(&mut self.variable_value);});
            if ui.button("▤ Задать переменную").clicked(){if let Ok(v)=self.variable_value.parse(){self.insert(Brick::SetVariable{name:self.variable_name.clone(),value:v});}else{self.status="Введите число для переменной".into();}}
            ui.add_space(8.0); ui.label(egui::RichText::new("УПРАВЛЕНИЕ").color(egui::Color32::from_rgb(100,210,160)).strong());
            if ui.button("◷ Ждать 1 секунду").clicked(){self.insert(Brick::Wait(1.0));}
            if ui.button("☁ Сказать сообщение").clicked(){self.insert(Brick::Say("Привет!".into()));}
        });
        egui::TopBottomPanel::bottom("status").exact_height(32.0).show(ctx, |ui| { ui.horizontal_centered(|ui| {ui.label("●"); ui.label(&self.status); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.label(format!("Кэш сцен: {}",self.manager.cached_scenes.len()));}); }); });
        egui::CentralPanel::default().show(ctx, |ui| {
            let name = self.current_def().map(|d|d.name.clone()).unwrap_or_default();
            ui.horizontal(|ui| {ui.heading(format!("{name} · Сцена")); ui.separator(); ui.label("Перетащи логику из палитры — добавляй кнопками");}); ui.separator();
            ui.columns(2, |cols| {
                let ui=&mut cols[0]; ui.label(egui::RichText::new("СЦЕНА").strong());
                let available=ui.available_size(); let (rect,_)=ui.allocate_exact_size(available,egui::Sense::hover());
                ui.painter().rect_filled(rect,egui::Rounding::same(12.0),egui::Color32::from_rgb(27,31,43));
                ui.painter().rect_stroke(rect,egui::Rounding::same(12.0),egui::Stroke::new(1.0,egui::Color32::from_gray(55)));
                if let Some(scene)=self.manager.current_scene.as_ref() {
                    for sprite in scene.sprites.iter().filter(|s|s.visible) {
                        let p=egui::pos2(rect.center().x+sprite.x,rect.center().y-sprite.y);
                        ui.painter().circle_filled(p,27.0,if sprite.id.contains("Звезда"){egui::Color32::from_rgb(245,190,75)}else{egui::Color32::from_rgb(160,125,220)});
                        ui.painter().text(p,egui::Align2::CENTER_CENTER,if sprite.id.contains("Звезда"){"★"}else{"🐈"},egui::FontId::proportional(25.0),egui::Color32::WHITE);
                        ui.painter().text(p+egui::vec2(0.0,38.0),egui::Align2::CENTER_CENTER,&sprite.id,egui::FontId::proportional(13.0),egui::Color32::LIGHT_GRAY);
                    }
                }
                let ui=&mut cols[1]; ui.label(egui::RichText::new("СКРИПТЫ · КИРПИЧИКИ").strong());
                egui::ScrollArea::vertical().show(ui,|ui| {
                    if let Some(def)=self.current_def() {
                        for script in &def.scripts {
                            ui.add_space(6.0);
                            let event_label=match script.event {Event::Start=>"⚑ Когда при старте",Event::Resume=>"↻ Когда возобновлена",Event::Touch=>"☝ Когда касание",Event::Always=>"∞ Всегда"};
                            brick_card(ui,event_label,egui::Color32::from_rgb(230,170,65),true);
                            for brick in &script.bricks {
                                let (text,color)=match brick {Brick::SetVariable{name,value}=>(format!("▤ задать {name} = {value}"),egui::Color32::from_rgb(165,115,225)),Brick::ChangeX(v)=>(format!("↗ изменить X на {v}"),egui::Color32::from_rgb(75,155,225)),Brick::ChangeY(v)=>(format!("↕ изменить Y на {v}"),egui::Color32::from_rgb(75,155,225)),Brick::SetGravity(v)=>(format!("⬇ гравитация {v}"),egui::Color32::from_rgb(75,155,225)),Brick::Bounce=>("⤴ отскок".into(),egui::Color32::from_rgb(75,155,225)),Brick::Wait(v)=>(format!("◷ ждать {v} сек"),egui::Color32::from_rgb(90,175,160)),Brick::Say(v)=>(format!("☁ сказать «{v}»"),egui::Color32::from_rgb(90,175,160))};
                                brick_card(ui,&text,color,false);
                            }
                        }
                    }
                    ui.add_space(12.0); ui.label("Переменные сцены");
                    if let Some(s)=self.manager.current_scene.as_ref() { for (key,value) in &s.global_variables { ui.label(format!("▤  {key}  =  {value}")); } }
                });
            });
        });
    }
}
fn brick_card(ui:&mut egui::Ui,text:&str,color:egui::Color32,event:bool){
    egui::Frame::default().fill(color.gamma_multiply(if event {0.3}else{0.22})).stroke(egui::Stroke::new(1.0,color.gamma_multiply(0.65))).rounding(egui::Rounding::same(8.0)).inner_margin(egui::Margin::symmetric(12.0,9.0)).show(ui,|ui|{ui.label(egui::RichText::new(text).strong().color(color));});
    ui.add_space(3.0);
}
fn main()->eframe::Result<()> {
    let options=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1120.0,740.0]).with_min_inner_size([800.0,600.0]),..Default::default()};
    eframe::run_native("PocketCat — визуальный конструктор",options,Box::new(|_|Ok(Box::<PocketCat>::default())))
}
