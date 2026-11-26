mod scene;
use eframe::egui;
use scene::{Brick, Event, NoopServices, SceneDefinition, SceneManager, SceneTransition, Sprite};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page { Projects, Editor, Categories, Bricks }
struct Category { name: &'static str, color: egui::Color32 }
const CATEGORIES: &[Category] = &[
    Category { name: "Добавлено в этом обновлении", color: egui::Color32::from_rgb(155,155,155) },
    Category { name: "Событие", color: egui::Color32::from_rgb(205,83,20) },
    Category { name: "Управление", color: egui::Color32::from_rgb(248,144,83) },
    Category { name: "Физика", color: egui::Color32::from_rgb(58,137,195) },
    Category { name: "Звуки и Видео", color: egui::Color32::from_rgb(142,65,181) },
    Category { name: "Образы", color: egui::Color32::from_rgb(102,158,72) },
    Category { name: "Частицы", color: egui::Color32::from_rgb(102,158,72) },
    Category { name: "Перо", color: egui::Color32::from_rgb(50,94,29) },
    Category { name: "Данные", color: egui::Color32::from_rgb(232,68,73) },
    Category { name: "Устройство", color: egui::Color32::from_rgb(151,112,7) },
    Category { name: "Поля для ввода текста", color: egui::Color32::from_rgb(102,158,72) },
    Category { name: "Мини сцены", color: egui::Color32::from_rgb(246,197,45) },
    Category { name: "Элементы интерфейса", color: egui::Color32::from_rgb(190,98,151) },
];

struct PocketCat {
    manager: SceneManager, selected: String, page: Page, category: String, event: Event,
    new_scene: String, variable_name: String, variable_value: String, x_value: String, y_value: String,
    status: String, running: bool, last_tick: std::time::Instant,
}
impl Default for PocketCat {
    fn default() -> Self {
        let sprite = |id: &str, x, y| Sprite { id: id.into(), x, y, costume: 0, visible: true, is_clone: false, local_variables: HashMap::new() };
        let defs = vec![
            SceneDefinition { id: "menu".into(), name: "Меню".into(), sprites: vec![sprite("Котик", 0.0, 0.0)], scripts: vec![scene::Script { event: Event::Start, bricks: vec![Brick::Say("Нажми Играть!".into())] }] },
            SceneDefinition { id: "level1".into(), name: "Режимы".into(), sprites: vec![sprite("Котик", -60.0, 0.0), sprite("Звезда", 70.0, 30.0)], scripts: vec![scene::Script { event: Event::Start, bricks: vec![Brick::SetVariable { name: "Очки".into(), value: 0.0 }] }] },
            SceneDefinition { id: "ending".into(), name: "Плохая концовка".into(), sprites: vec![sprite("Игрок", 0.0, 0.0)], scripts: vec![scene::Script { event: Event::Start, bricks: vec![] }] },
        ];
        let mut manager = SceneManager::new(defs);
        manager.transition("menu", SceneTransition::StartScene, &mut NoopServices).unwrap();
        Self { manager, selected: "menu".into(), page: Page::Projects, category: "Физика".into(), event: Event::Start,
            new_scene: String::new(), variable_name: "Очки".into(), variable_value: "0".into(), x_value: "100".into(), y_value: "200".into(),
            status: "Проект готов к редактированию".into(), running: false, last_tick: std::time::Instant::now() }
    }
}
impl PocketCat {
    fn insert(&mut self, brick: Brick) {
        match self.manager.add_brick(&self.selected, self.event, brick) { Ok(()) => self.status = "Кирпичик добавлен".into(), Err(e) => self.status = e }
        self.page = Page::Editor;
    }
    fn current_def(&self) -> Option<&SceneDefinition> { self.manager.scene_definitions().find(|d| d.id == self.selected) }
    fn set_scene(&mut self, id: String, fresh: bool) {
        let transition = if fresh { SceneTransition::StartScene } else { SceneTransition::ResumeScene };
        match self.manager.transition(&id, transition, &mut NoopServices) {
            Ok(()) => { self.selected = id; self.status = if fresh { "Сцена запущена с нуля" } else { "Сцена продолжена из памяти" }.into(); },
            Err(e) => self.status = e,
        }
    }
    fn new_scene(&mut self) {
        let name = self.new_scene.trim().to_owned(); if name.is_empty() { return; }
        let id = format!("scene{}", self.manager.scene_definitions().count() + 1);
        let d = SceneDefinition { id: id.clone(), name, sprites: vec![Sprite { id: "Котик".into(), x:0.0,y:0.0,costume:0,visible:true,is_clone:false,local_variables:HashMap::new() }], scripts: vec![scene::Script { event: Event::Start, bricks: vec![] }] };
        if self.manager.add_scene(d).is_ok() { self.new_scene.clear(); self.set_scene(id, true); self.page = Page::Editor; }
    }
}
impl eframe::App for PocketCat {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(0, 43, 56);
        visuals.window_fill = egui::Color32::from_rgb(0, 52, 66);
        ctx.set_visuals(visuals);
        if self.running { let now=std::time::Instant::now(); self.manager.tick(now.duration_since(self.last_tick).as_secs_f64().min(0.1)); self.last_tick=now; ctx.request_repaint(); }
        else { self.last_tick=std::time::Instant::now(); }
        let title = match self.page { Page::Projects => "Pocket Up".to_string(), Page::Editor => self.current_def().map(|d|d.name.clone()).unwrap_or_else(||"Сцена".into()), Page::Categories => "Категории".into(), Page::Bricks => self.category.clone() };
        egui::TopBottomPanel::top("top").exact_height(60.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if self.page != Page::Projects && ui.button(egui::RichText::new("‹").size(38.0)).clicked() {
                    self.page = match self.page { Page::Bricks=>Page::Categories, Page::Categories=>Page::Editor, _=>Page::Projects };
                }
                ui.add_space(12.0); ui.label(egui::RichText::new(title).size(22.0).color(egui::Color32::from_rgb(223,241,248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.page == Page::Editor {
                        if ui.button(if self.running { "■" } else { "▶" }).clicked() { self.running=!self.running; }
                        if ui.button("⋮").clicked() { self.manager.reset_scene_cache(&self.selected,&mut NoopServices); self.set_scene(self.selected.clone(),true); }
                    } else { ui.label(egui::RichText::new("⋮").size(24.0)); }
                });
            });
        });
        egui::TopBottomPanel::bottom("bottom").exact_height(34.0).show(ctx,|ui|{
            ui.horizontal_centered(|ui|{ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(125,218,172)));ui.label(&self.status);ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.label(format!("{} сцен",self.manager.scene_definitions().count()));});});
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Projects => self.projects(ui),
            Page::Editor => self.editor(ui),
            Page::Categories => self.categories(ui),
            Page::Bricks => self.bricks(ui),
        });
    }
}
impl PocketCat {
    fn projects(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(egui::RichText::new("Мои проекты  ›  PocketCat").size(15.0).color(egui::Color32::LIGHT_BLUE));
            ui.add_space(10.0);
            let scenes: Vec<(String,String)> = self.manager.scene_definitions().map(|d|(d.id.clone(),d.name.clone())).collect();
            for (idx,(id,name)) in scenes.iter().enumerate() {
                let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),112.0),egui::Sense::click());
                ui.painter().rect_filled(rect,0.0,egui::Color32::from_rgb(0,73,91));
                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min+egui::vec2(54.0,13.0),egui::vec2(92.0,84.0)),0.0,scene_preview_color(idx));
                ui.painter().text(rect.min+egui::vec2(100.0,55.0),egui::Align2::LEFT_CENTER,name,egui::FontId::proportional(21.0),egui::Color32::from_rgb(175,217,236));
                ui.painter().text(rect.right_top()+egui::vec2(-30.0,25.0),egui::Align2::CENTER_CENTER,"⋮",egui::FontId::proportional(26.0),egui::Color32::LIGHT_BLUE);
                if response.clicked() { self.set_scene(id.clone(),false); self.page=Page::Editor; }
                ui.add_space(2.0);
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| { ui.add_sized([ui.available_width()-58.0,42.0],egui::TextEdit::singleline(&mut self.new_scene).hint_text("Имя новой сцены")); if ui.add_sized([48.0,42.0],egui::Button::new(egui::RichText::new("＋").size(25.0))).clicked(){self.new_scene();} });
            ui.add_space(10.0);
            ui.horizontal(|ui| { if ui.button("▶  Играть").clicked(){self.set_scene("menu".into(),true);self.running=true;} if ui.button("＋  Новая сцена").clicked(){self.new_scene();} });
        });
    }
    fn editor(&mut self, ui:&mut egui::Ui) {
        ui.add_space(7.0);
        ui.horizontal(|ui|{ui.label(egui::RichText::new("СЦЕНА").strong());ui.separator();ui.label(egui::RichText::new("Объекты").weak());});
        let stage_h=(ui.available_height()*0.35).clamp(160.0,260.0);
        let (rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),stage_h),egui::Sense::click());
        ui.painter().rect_filled(rect,0.0_f32,egui::Color32::from_rgb(242,245,244));
        ui.painter().rect_stroke(rect,0.0_f32,egui::Stroke::new(1.0,egui::Color32::from_gray(150)));
        if let Some(s)=self.manager.current_scene.as_ref(){ for sp in s.sprites.iter().filter(|s|s.visible){let p=rect.center()+egui::vec2(sp.x,-sp.y);ui.painter().circle_filled(p,24.0,egui::Color32::from_rgb(164,203,79));ui.painter().text(p,egui::Align2::CENTER_CENTER,if sp.id.contains("Звезда"){"★"}else{"🐈"},egui::FontId::proportional(25.0),egui::Color32::WHITE);}}
        if let Some(scene)=self.manager.current_scene.as_ref(){ui.horizontal_wrapped(|ui|{for sp in &scene.sprites{ui.label(egui::RichText::new(format!("🐈 {}",sp.id)).color(egui::Color32::LIGHT_BLUE));}});}
        ui.separator();
        ui.horizontal(|ui|{ui.label(egui::RichText::new("Сценарии").strong());ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.button("＋ Блоки").clicked(){self.page=Page::Categories;}});});
        egui::ScrollArea::vertical().show(ui,|ui|{
            if let Some(def)=self.current_def(){let scripts=def.scripts.clone();for script in scripts{
                let (label,color)=event_card(script.event); brick_card(ui,label,color);
                for brick in script.bricks {let (label,color)=brick_card_info(&brick);brick_card(ui,&label,color);}
                ui.add_space(7.0);
            }}
            if let Some(s)=self.manager.current_scene.as_ref(){ui.label("Переменные");for(k,v)in &s.global_variables{ui.label(format!("▤ {k} = {v}"));}}
        });
    }
    fn categories(&mut self,ui:&mut egui::Ui){
        egui::ScrollArea::vertical().show(ui,|ui|{for c in CATEGORIES{let (r,resp)=ui.allocate_exact_size(egui::vec2(ui.available_width(),72.0),egui::Sense::click());ui.painter().rect_filled(r,0.0_f32,c.color);ui.painter().line_segment([r.left_bottom(),r.right_bottom()],egui::Stroke::new(2.0,egui::Color32::from_rgb(26,31,32)));ui.painter().text(r.min+egui::vec2(24.0,36.0),egui::Align2::LEFT_CENTER,c.name,egui::FontId::proportional(23.0),egui::Color32::from_rgb(244,239,232));if resp.clicked(){self.category=c.name.into();self.page=Page::Bricks;}ui.add_space(3.0);}});
    }
    fn bricks(&mut self,ui:&mut egui::Ui){
        egui::ScrollArea::vertical().show(ui,|ui|match self.category.as_str(){
            "Событие"=>{
                for (event,label) in [(Event::Start,"При запуске сцены"),(Event::Resume,"При возобновлении сцены"),(Event::Touch,"При нажатии на объект"),(Event::Always,"Всегда")]{if brick_button(ui,label,egui::Color32::from_rgb(204,83,21)){self.event=event;self.page=Page::Editor;self.status=format!("Выбрано событие: {label}");}}
                for label in ["Нажали клавишу и сохранить ее имя","Отпустили клавишу и сохранить ее имя","При нажатии на кнопку назад","При перемещении по объекту","При отпускании от объекта"]{brick_button(ui,label,egui::Color32::from_rgb(204,83,21));}
            }
            "Физика"=>{
                ui.label(egui::RichText::new("ПОМЕСТИТЬ В").strong());
                ui.horizontal(|ui|{ui.label("X:");ui.text_edit_singleline(&mut self.x_value);ui.label("Y:");ui.text_edit_singleline(&mut self.y_value);});
                if brick_button(ui,&format!("Установить X в  {}",self.x_value),egui::Color32::from_rgb(58,137,195)){if let Ok(v)=self.x_value.parse(){self.insert(Brick::SetX(v));}}
                if brick_button(ui,&format!("Установить Y в  {}",self.y_value),egui::Color32::from_rgb(58,137,195)){if let Ok(v)=self.y_value.parse(){self.insert(Brick::SetY(v));}}
                if brick_button(ui,"Изменить X на: 10",egui::Color32::from_rgb(58,137,195)){self.insert(Brick::ChangeX(10.0));}
                if brick_button(ui,"Изменить Y на: 10",egui::Color32::from_rgb(58,137,195)){self.insert(Brick::ChangeY(10.0));}
                if brick_button(ui,"Гравитация  −9.8",egui::Color32::from_rgb(58,137,195)){self.insert(Brick::SetGravity(-9.8));}
                if brick_button(ui,"Отскок",egui::Color32::from_rgb(58,137,195)){self.insert(Brick::Bounce);}
                if brick_button(ui,"Перейти в сцену…",egui::Color32::from_rgb(58,137,195)){self.insert(Brick::Say("Переход между сценами".into()));}
            }
            "Мини сцены"=>{
                let list:Vec<(String,String)>=self.manager.scene_definitions().map(|d|(d.id.clone(),d.name.clone())).collect();
                for (id,name) in list {if brick_button(ui,&format!("Запустить сцену  {name}"),egui::Color32::from_rgb(245,194,45)){self.set_scene(id.clone(),true);self.page=Page::Editor;} if brick_button(ui,&format!("Продолжить сцену  {name}"),egui::Color32::from_rgb(245,194,45)){self.set_scene(id,false);self.page=Page::Editor;}}
            }
            "Данные"=>{
                ui.horizontal(|ui|{ui.text_edit_singleline(&mut self.variable_name);ui.text_edit_singleline(&mut self.variable_value);});
                if brick_button(ui,"Создать / задать переменную",egui::Color32::from_rgb(232,68,73)){if let Ok(value)=self.variable_value.parse(){self.insert(Brick::SetVariable{name:self.variable_name.clone(),value});}}
            }
            "Управление"=>{if brick_button(ui,"Ждать 1 секунду",egui::Color32::from_rgb(248,144,83)){self.insert(Brick::Wait(1.0));} if brick_button(ui,"Повторять всегда",egui::Color32::from_rgb(248,144,83)){self.insert(Brick::Say("Повторять всегда".into()));}}
            "Звуки и Видео"=>{for label in ["Воспроизвести звук","Воспроизвести звук и ждать","Остановить звук","Остановить все звуки","Задать громкость"]{brick_button(ui,label,egui::Color32::from_rgb(142,65,181));}}
            "Образы"=>{for label in ["Следующий образ","Переключить образ","Изменить размер","Показаться","Спрятаться"]{brick_button(ui,label,egui::Color32::from_rgb(102,158,72));}}
            _=>{ui.label("Кирпичики категории появятся здесь.");}
        });
    }
}
fn scene_preview_color(i:usize)->egui::Color32{match i%4{0=>egui::Color32::from_rgb(78,96,88),1=>egui::Color32::from_rgb(115,8,36),2=>egui::Color32::from_rgb(245,245,240),_=>egui::Color32::from_rgb(72,8,32)}}
fn event_card(e:Event)->(&'static str,egui::Color32){(match e{Event::Start=>"При запуске сцены",Event::Resume=>"При возобновлении сцены",Event::Touch=>"При нажатии на объект",Event::Always=>"Всегда"},egui::Color32::from_rgb(204,83,21))}
fn brick_card_info(b:&Brick)->(String,egui::Color32){match b{Brick::SetVariable{name,value}=>(format!("Задать {name} в {value}"),egui::Color32::from_rgb(220,75,78)),Brick::SetX(v)=>(format!("Установить X в {v}"),egui::Color32::from_rgb(58,137,195)),Brick::SetY(v)=>(format!("Установить Y в {v}"),egui::Color32::from_rgb(58,137,195)),Brick::ChangeX(v)=>(format!("Изменить X на {v}"),egui::Color32::from_rgb(58,137,195)),Brick::ChangeY(v)=>(format!("Изменить Y на {v}"),egui::Color32::from_rgb(58,137,195)),Brick::SetGravity(v)=>(format!("Гравитация {v}"),egui::Color32::from_rgb(58,137,195)),Brick::Bounce=>("Отскок".into(),egui::Color32::from_rgb(58,137,195)),Brick::Wait(v)=>(format!("Ждать {v} секунд"),egui::Color32::from_rgb(248,144,83)),Brick::Say(v)=>(format!("Сказать {v}"),egui::Color32::from_rgb(248,144,83))}}
fn brick_card(ui:&mut egui::Ui,label:&str,color:egui::Color32){let (r,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),54.0),egui::Sense::hover());ui.painter().rect_filled(r,5.0_f32,color);ui.painter().rect_stroke(r,5.0_f32,egui::Stroke::new(1.0,color.gamma_multiply(0.8)));ui.painter().text(r.min+egui::vec2(48.0,27.0),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(17.0),egui::Color32::WHITE);ui.add_space(5.0);}
fn brick_button(ui:&mut egui::Ui,label:&str,color:egui::Color32)->bool{let (r,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),74.0),egui::Sense::click());ui.painter().rect_filled(r,3.0_f32,color);ui.painter().line_segment([r.left_top(),r.right_top()],egui::Stroke::new(2.0,color.gamma_multiply(1.4_f32)));ui.painter().text(r.min+egui::vec2(62.0,37.0),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(19.0),egui::Color32::WHITE);ui.add_space(4.0);response.clicked()}

fn native_options()->eframe::NativeOptions{eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([420.0,850.0]).with_min_inner_size([340.0,560.0]),..Default::default()}}
#[cfg(not(target_os="android"))]
fn main()->eframe::Result<()>{eframe::run_native("PocketCat",native_options(),Box::new(|_|Ok(Box::<PocketCat>::default())))}
#[cfg(target_os="android")]
pub fn run_android(app:android_activity::AndroidApp){use winit::platform::android::EventLoopBuilderExtAndroid;let options=eframe::NativeOptions{event_loop_builder:Some(Box::new(move|b|{b.with_android_app(app);})),..native_options()};let _=eframe::run_native("PocketCat",options,Box::new(|_|Ok(Box::<PocketCat>::default())));}
