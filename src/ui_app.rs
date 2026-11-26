use crate::scene::{Brick, Event, NoopServices, SceneDefinition, SceneManager, SceneTransition, Sprite};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};
slint::include_modules!();

struct AppState {
    manager: SceneManager,
    selected: String,
    event: Event,
    category: String,
}

impl AppState {
    fn new() -> Self {
        let sprite = |id: &str, x, y| Sprite { id: id.into(), x, y, costume: 0, visible: true, is_clone: false, local_variables: HashMap::new() };
        let definitions = vec![
            SceneDefinition { id: "menu".into(), name: "Меню".into(), sprites: vec![sprite("Котик", 0.0, 0.0)], scripts: vec![crate::scene::Script { event: Event::Start, bricks: vec![Brick::Say("Нажми Играть!".into())] }] },
            SceneDefinition { id: "level1".into(), name: "Режимы".into(), sprites: vec![sprite("Котик", -60.0, 0.0), sprite("Звезда", 70.0, 30.0)], scripts: vec![crate::scene::Script { event: Event::Start, bricks: vec![Brick::SetVariable { name: "Очки".into(), value: 0.0 }] }] },
            SceneDefinition { id: "ending".into(), name: "Плохая концовка".into(), sprites: vec![sprite("Игрок", 0.0, 0.0)], scripts: vec![crate::scene::Script { event: Event::Start, bricks: vec![] }] },
        ];
        let mut manager = SceneManager::new(definitions);
        let _ = manager.transition("menu", SceneTransition::StartScene, &mut NoopServices);
        Self { manager, selected: "menu".into(), event: Event::Start, category: String::new() }
    }
}

fn color(r: u8, g: u8, b: u8) -> slint::Color { slint::Color::from_rgb_u8(r, g, b) }
#[derive(Deserialize)]
struct CatalogCategory { id: String, title: String, color: String, blocks: Vec<CatalogBlock> }
#[derive(Deserialize)]
struct CatalogBlock { id: String, title: String }

fn block_catalog() -> Vec<CatalogCategory> {
    serde_json::from_str(include_str!("../ui/block-catalog.json"))
        .expect("Pocket Up block catalog must be valid JSON")
}

#[cfg(test)]
mod catalog_tests {
    use super::block_catalog;

    #[test]
    fn contains_the_complete_upstream_palette() {
        let catalog = block_catalog();
        assert_eq!(catalog.len(), 14);
        assert_eq!(catalog.iter().map(|category| category.blocks.len()).sum::<usize>(), 243);
        assert!(catalog.iter().all(|category| !category.blocks.is_empty()));
    }
}

fn parse_color(value: &str) -> slint::Color {
    let rgb = u32::from_str_radix(value.trim_start_matches('#'), 16).unwrap_or(0x607d8b);
    color(((rgb >> 16) & 0xff) as u8, ((rgb >> 8) & 0xff) as u8, (rgb & 0xff) as u8)
}

fn categories() -> Vec<CategoryItem> {
    block_catalog().into_iter().map(|category| CategoryItem {
        title: category.title.into(), color: parse_color(&category.color),
    }).collect()
}

fn refresh(ui: &MainWindow, state: &AppState) {
    let scenes: Vec<SceneItem> = state.manager.scene_definitions().map(|d| SceneItem { id: d.id.clone().into(), title: d.name.clone().into() }).collect();
    ui.set_scenes(ModelRc::new(VecModel::from(scenes)));
    let title = match ui.get_page().as_str() {
        "projects" => "Pocket Up".to_owned(),
        "editor" => state.manager.current_scene.as_ref().map(|s| s.definition.name.clone()).unwrap_or_else(|| "Сцена".into()),
        "categories" => "Категории".into(),
        "blocks" => state.category.clone(),
        _ => "Pocket Up".into(),
    };
    ui.set_header_title(title.into());
    if let Some(scene) = state.manager.current_scene.as_ref() {
        ui.set_sprites(ModelRc::new(VecModel::from(scene.sprites.iter().map(|s| SharedString::from(s.id.clone())).collect::<Vec<_>>())));
        let mut scripts = Vec::new();
        for script in &scene.definition.scripts {
            let title = match script.event { Event::Start=>"При запуске сцены", Event::Resume=>"При возобновлении сцены", Event::Touch=>"При нажатии на объект", Event::Always=>"Всегда" };
            scripts.push(BlockItem { id: format!("event:{:?}",script.event).into(), title: title.into(), color: color(205,83,20) });
            for brick in &script.bricks { let (text, c) = brick_info(brick); scripts.push(BlockItem { id: String::new().into(), title: text.into(), color: c }); }
        }
        ui.set_scripts(ModelRc::new(VecModel::from(scripts)));
    }
}

fn brick_info(brick: &Brick) -> (String, slint::Color) {
    match brick {
        Brick::StartScene(id) => (format!("Запустить сцену {id}"),color(246,197,45)),
        Brick::ResumeScene(id) => (format!("Продолжить сцену {id}"),color(246,197,45)),
        Brick::SetVariable{name,value} => (format!("Задать {name} в {value}"),color(232,68,73)),
        Brick::SetX(v) => (format!("Установить X в {v}"),color(58,137,195)),
        Brick::SetY(v) => (format!("Установить Y в {v}"),color(58,137,195)),
        Brick::ChangeX(v) => (format!("Изменить X на {v}"),color(58,137,195)),
        Brick::ChangeY(v) => (format!("Изменить Y на {v}"),color(58,137,195)),
        Brick::SetGravity(v) => (format!("Гравитация {v}"),color(58,137,195)),
        Brick::Bounce => ("Отскок".into(),color(58,137,195)),
        Brick::Wait(v) => (format!("Ждать {v} секунд"),color(248,144,83)),
        Brick::Say(v) => (format!("Сказать {v}"),color(248,144,83)),
    }
}

fn blocks_for(category: &str) -> Vec<BlockItem> {
    block_catalog().into_iter()
        .find(|item| item.title == category)
        .map(|item| {
            let color = parse_color(&item.color);
            item.blocks.into_iter().map(|block| BlockItem {
                id: format!("catalog:{}:{}", item.id, block.id).into(),
                title: block.title.into(), color,
            }).collect()
        })
        .unwrap_or_default()
}

fn choose_category(ui: &MainWindow, state: &mut AppState, name: &str) {
    state.category = name.to_owned();
    ui.set_blocks(ModelRc::new(VecModel::from(blocks_for(name))));
    ui.set_page("blocks".into());
    ui.set_header_title(name.into());
}

fn add_block(ui: &MainWindow, state: &mut AppState, id: &str) {
    if id.starts_with("catalog:") {
        ui.set_status("Блок из каталога Pocket Up: выполнение пока не подключено".into());
        refresh(ui, state);
        return;
    }
    match id {
        "event:start" => { state.event=Event::Start; ui.set_page("editor".into()); }
        "event:resume" => { state.event=Event::Resume; ui.set_page("editor".into()); }
        "event:touch" => { state.event=Event::Touch; ui.set_page("editor".into()); }
        "event:always" => { state.event=Event::Always; ui.set_page("editor".into()); }
        "event:key" | "event:back" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::Say(format!("{} event",id))); ui.set_page("editor".into()); }
        "set-x" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::SetX(100.0)); }
        "set-y" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::SetY(200.0)); }
        "change-x" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::ChangeX(10.0)); }
        "change-y" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::ChangeY(10.0)); }
        "gravity" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::SetGravity(-9.8)); }
        "bounce" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::Bounce); }
        "wait" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::Wait(1.0)); }
        "repeat" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::Say("Повторять всегда".into())); }
        "variable" => { let _=state.manager.add_brick(&state.selected,state.event,Brick::SetVariable{name:"Очки".into(),value:0.0}); }
        _ if id.starts_with("start-scene:") => { let target=&id[12..];let _=state.manager.add_brick(&state.selected,state.event,Brick::StartScene(target.to_owned())); }
        _ if id.starts_with("resume-scene:") => { let target=&id[13..];let _=state.manager.add_brick(&state.selected,state.event,Brick::ResumeScene(target.to_owned())); }
        _ => {}
    }
    if ui.get_page() == "blocks" { ui.set_page("editor".into()); }
    refresh(ui,state);
}

fn create_scene(ui: &MainWindow, state: &mut AppState, name: &str) {
    let name=name.trim(); if name.is_empty(){return;}
    let id=format!("scene{}",state.manager.scene_definitions().count()+1);
    let definition=SceneDefinition{id:id.clone(),name:name.into(),sprites:vec![Sprite{id:"Котик".into(),x:0.0,y:0.0,costume:0,visible:true,is_clone:false,local_variables:HashMap::new()}],scripts:vec![crate::scene::Script{event:Event::Start,bricks:vec![]}]};
    if state.manager.add_scene(definition).is_ok(){let _=state.manager.transition(&id,SceneTransition::StartScene,&mut NoopServices);state.selected=id;ui.set_new_scene_name("".into());ui.set_page("editor".into());ui.set_status("Новая сцена создана".into());refresh(ui,state);}
}

fn connect_callbacks(ui: &MainWindow, state: Rc<RefCell<AppState>>, timer: Rc<Timer>) {
    let weak=ui.as_weak(); let s=state.clone(); ui.on_back(move || {if let Some(ui)=weak.upgrade(){let page=ui.get_page();let state=s.borrow();let next=if page=="blocks"{"categories"}else if page=="categories"{"editor"}else{"projects"};ui.set_page(next.into());refresh(&ui,&state);}});
    let weak=ui.as_weak();let s=state.clone();ui.on_open_scene(move|id|{if let Some(ui)=weak.upgrade(){let mut state=s.borrow_mut();let id=id.to_string();if state.manager.transition(&id,SceneTransition::ResumeScene,&mut NoopServices).is_ok(){state.selected=id;ui.set_page("editor".into());ui.set_status("Сцена продолжена из памяти".into());refresh(&ui,&state);}}});
    let weak=ui.as_weak();let s=state.clone();ui.on_create_scene(move|name|{if let Some(ui)=weak.upgrade(){create_scene(&ui,&mut s.borrow_mut(),name.as_str());}});
    let weak=ui.as_weak();let s=state.clone();ui.on_open_category(move|name|{if let Some(ui)=weak.upgrade(){choose_category(&ui,&mut s.borrow_mut(),name.as_str());}});
    let weak=ui.as_weak();let s=state.clone();ui.on_add_block(move|id|{if let Some(ui)=weak.upgrade(){add_block(&ui,&mut s.borrow_mut(),id.as_str());}});
    let weak=ui.as_weak();let s=state.clone();let t=timer.clone();ui.on_toggle_run(move||{if let Some(ui)=weak.upgrade(){let run=!ui.get_running();ui.set_running(run);if run{ui.set_status("Игра выполняется".into());let state=s.clone();let weak_ui=ui.as_weak();t.start(TimerMode::Repeated,Duration::from_millis(16),move||{let mut state=state.borrow_mut();state.manager.tick(0.016,&mut NoopServices);if let Some(current)=state.manager.current_scene.as_ref(){state.selected=current.definition.id.clone();}if let Some(ui)=weak_ui.upgrade(){refresh(&ui,&state);}});}else{t.stop();ui.set_status("Игра остановлена".into());}}});
    let weak=ui.as_weak();let s=state.clone();ui.on_reset_scene(move||{if let Some(ui)=weak.upgrade(){let mut state=s.borrow_mut();let id=state.selected.clone();state.manager.reset_scene_cache(&id,&mut NoopServices);let _=state.manager.transition(&id,SceneTransition::StartScene,&mut NoopServices);ui.set_status("Сцена сброшена".into());refresh(&ui,&state);}});
    let weak=ui.as_weak();let s=state.clone();ui.on_save_project(move||{if let Some(ui)=weak.upgrade(){let state=s.borrow();let result=state.manager.export_project_json().and_then(|data|std::fs::write("PocketCat-project.json",data).map_err(|e|e.to_string()));ui.set_status(match result{Ok(())=>"Сохранено: PocketCat-project.json".into(),Err(e)=>format!("Ошибка сохранения: {e}").into()});}});
    let weak=ui.as_weak();let s=state;ui.on_load_project(move||{if let Some(ui)=weak.upgrade(){let mut state=s.borrow_mut();let result=std::fs::read_to_string("PocketCat-project.json").map_err(|e|e.to_string()).and_then(|data|state.manager.import_project_json(&data,&mut NoopServices));match result{Ok(())=>{if let Some(scene)=state.manager.current_scene.as_ref(){state.selected=scene.definition.id.clone();}ui.set_status("Проект загружен".into());refresh(&ui,&state);},Err(e)=>ui.set_status(format!("Ошибка загрузки: {e}").into())}}});
}

pub fn run() -> Result<(), slint::PlatformError> {
    let ui=MainWindow::new()?;
    let state=Rc::new(RefCell::new(AppState::new()));
    let timer=Rc::new(Timer::default());
    ui.set_scenes(ModelRc::new(VecModel::from(Vec::<SceneItem>::new())));
    ui.set_categories(ModelRc::new(VecModel::from(categories())));
    ui.set_blocks(ModelRc::new(VecModel::from(Vec::<BlockItem>::new())));
    ui.set_sprites(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
    ui.set_scripts(ModelRc::new(VecModel::from(Vec::<BlockItem>::new())));
    refresh(&ui,&state.borrow());
    connect_callbacks(&ui,state,timer);
    ui.run()
}
