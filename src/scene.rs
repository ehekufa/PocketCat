//! Scene lifecycle and cache for the PocketCat runtime.
use std::collections::HashMap;

pub type SceneId = String;

fn default_sprite_size() -> f32 { 100.0 }
fn default_sprite_alpha() -> f32 { 100.0 }
fn default_sprite_tint() -> [u8; 3] { [145, 201, 77] }

fn hue_to_rgb(hue: f32) -> [u8; 3] {
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h.rem_euclid(2.0) - 1.0).abs();
    let (r, g, b) = match h as u8 {
        0 => (1.0, x, 0.0), 1 => (x, 1.0, 0.0), 2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0), 4 => (x, 0.0, 1.0), _ => (1.0, 0.0, x),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SceneTransition { StartScene, ResumeScene }

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sprite {
    pub id: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub velocity_x: f32,
    #[serde(default)]
    pub velocity_y: f32,
    #[serde(default = "default_sprite_size")]
    pub size: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "default_sprite_alpha")]
    pub alpha: f32,
    #[serde(default = "default_sprite_alpha")]
    pub brightness: f32,
    #[serde(default = "default_sprite_tint")]
    pub tint: [u8; 3],
    pub costume: usize,
    pub visible: bool,
    pub is_clone: bool,
    pub local_variables: HashMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Brick {
    SetVariable { name: String, value: f64 },
    StartScene(String),
    ResumeScene(String),
    SetX(f32),
    SetY(f32),
    ChangeX(f32),
    ChangeY(f32),
    SetGravity(f32),
    Bounce,
    Wait(f32),
    Say(String),
    /// A Pocket Up palette command. The interpreter implements supported command families
    /// while preserving the original Pocket Up identifier in saved projects.
    PocketUp { id: String, title: String, #[serde(default)] arguments: Vec<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Event { Start, Resume, Touch, Always }

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Script { pub event: Event, pub bricks: Vec<Brick> }

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SceneDefinition {
    pub id: SceneId,
    pub name: String,
    pub sprites: Vec<Sprite>,
    pub scripts: Vec<Script>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneLifecycle { Loaded, Active, Cached, Unloaded }

fn pocket_block_id(brick: &Brick) -> Option<&str> {
    if let Brick::PocketUp { id, .. } = brick { Some(id) } else { None }
}

fn matching_end(program: &[Brick], start: usize, opening: &str, closing: &str) -> Option<usize> {
    let mut nesting = 0usize;
    for index in start + 1..program.len() {
        match pocket_block_id(&program[index]) {
            Some(id) if id == opening => nesting += 1,
            Some(id) if id == closing => {
                if nesting == 0 { return Some(index); }
                nesting -= 1;
            }
            _ => {}
        }
    }
    None
}

fn matching_if(program: &[Brick], start: usize) -> (Option<usize>, usize) {
    let mut nesting = 0usize;
    let mut otherwise = None;
    for index in start + 1..program.len() {
        match pocket_block_id(&program[index]) {
            Some("if" | "if (2)" | "ifElse (2)") => nesting += 1,
            Some("endIf") if nesting == 0 => return (otherwise, index),
            Some("endIf") => nesting -= 1,
            Some("else") if nesting == 0 && otherwise.is_none() => otherwise = Some(index),
            _ => {}
        }
    }
    (otherwise, program.len().saturating_sub(1))
}

fn evaluate_condition(arguments: &[String]) -> bool {
    let left = arguments.first().and_then(|value| value.parse::<f64>().ok()).unwrap_or(1.0);
    let operator = arguments.get(1).map(String::as_str).unwrap_or("<");
    let right = arguments.get(2).and_then(|value| value.parse::<f64>().ok()).unwrap_or(2.0);
    match operator {
        "=" | "==" => (left - right).abs() < f64::EPSILON,
        "!=" | "~=" => (left - right).abs() >= f64::EPSILON,
        ">" => left > right,
        ">=" => left >= right,
        "<=" => left <= right,
        _ => left < right,
    }
}

/// Platform services which must be silenced when a scene is deactivated.
pub trait SceneServices {
    fn stop_scene_audio(&mut self, scene: &str);
    fn close_scene_dialog_and_keyboard(&mut self, scene: &str);
    fn disable_scene_input_and_sensors(&mut self, scene: &str);
    fn pause_scene_bricks(&mut self, scene: &str);
    fn unload_scene_assets(&mut self, scene: &str);
}

#[derive(Default)]
pub struct NoopServices;
impl SceneServices for NoopServices {
    fn stop_scene_audio(&mut self, _: &str) {}
    fn close_scene_dialog_and_keyboard(&mut self, _: &str) {}
    fn disable_scene_input_and_sensors(&mut self, _: &str) {}
    fn pause_scene_bricks(&mut self, _: &str) {}
    fn unload_scene_assets(&mut self, _: &str) {}
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub definition: SceneDefinition,
    pub sprites: Vec<Sprite>,
    pub global_variables: HashMap<String, f64>,
    pub arrays: HashMap<String, Vec<String>>,
    pub lifecycle: SceneLifecycle,
    pub elapsed_seconds: f64,
    pub start_events: u64,
    pub resume_events: u64,
    pub touch_enabled: bool,
    pub gravity: f32,
    pending_transition: Option<(SceneId, SceneTransition)>,
}

impl Scene {
    fn load(definition: SceneDefinition) -> Self {
        let scene = Self {
            sprites: definition.sprites.clone(), definition,
            global_variables: HashMap::new(), arrays: HashMap::new(), lifecycle: SceneLifecycle::Loaded,
            elapsed_seconds: 0.0, start_events: 0,
            resume_events: 0, touch_enabled: false, gravity: -9.8, pending_transition: None,
        };
        scene
    }

    fn activate(&mut self, resumed: bool) {
        self.lifecycle = SceneLifecycle::Active;
        self.touch_enabled = true;
        if resumed { self.resume_events += 1; self.run_event(Event::Resume); }
        else { self.start_events += 1; self.run_event(Event::Start); }
    }

    fn run_event(&mut self, event: Event) {
        let programs: Vec<Vec<Brick>> = self.definition.scripts.iter()
            .filter(|script| script.event == event).map(|script| script.bricks.clone()).collect();
        for program in programs { self.execute_program(&program, 0); }
    }

    fn execute_program(&mut self, program: &[Brick], depth: usize) {
        if depth > 16 { return; }
        let mut pc = 0;
        let mut budget = 10_000usize;
        while pc < program.len() && budget > 0 {
            budget -= 1;
            let Brick::PocketUp { id, arguments, .. } = &program[pc] else {
                self.apply(program[pc].clone()); pc += 1; continue;
            };
            match id.as_str() {
                "repeat" | "for" | "foreach" => {
                    let closing = match id.as_str() { "for" => "endFor", "foreach" => "endForeach", _ => "endRepeat" };
                    let end = matching_end(program, pc, id, closing);
                    if let Some(end) = end {
                        let count = arguments.first().and_then(|v| v.parse::<usize>().ok()).unwrap_or(10).min(1000);
                        for _ in 0..count { self.execute_program(&program[pc + 1..end], depth + 1); }
                        pc = end + 1;
                    } else { self.apply(program[pc].clone()); pc += 1; }
                }
                "cycleForever" | "repeatIsTrue" => {
                    let closing = if id == "cycleForever" { "endCycleForever" } else { "endRepeat" };
                    if let Some(end) = matching_end(program, pc, id, closing) {
                        let count = if id == "cycleForever" || evaluate_condition(arguments) { 64 } else { 0 };
                        for _ in 0..count { self.execute_program(&program[pc + 1..end], depth + 1); }
                        pc = end + 1;
                    } else { self.apply(program[pc].clone()); pc += 1; }
                }
                "waitIfTrue" | "waitIfTrue2" => {
                    let end = matching_end(program, pc, id, "endWait");
                    if let Some(end) = end {
                        if evaluate_condition(arguments) { self.execute_program(&program[pc + 1..end], depth + 1); }
                        pc = end + 1;
                    } else { self.apply(program[pc].clone()); pc += 1; }
                }
                "if" | "if (2)" | "ifElse (2)" => {
                    let (otherwise, end) = matching_if(program, pc);
                    if evaluate_condition(arguments) {
                        let finish = otherwise.unwrap_or(end);
                        self.execute_program(&program[pc + 1..finish], depth + 1);
                    } else if let Some(otherwise) = otherwise {
                        self.execute_program(&program[otherwise + 1..end], depth + 1);
                    }
                    pc = end.saturating_add(1);
                }
                "else" | "endIf" | "endRepeat" | "endCycleForever" | "endFor" | "endForeach" => pc += 1,
                _ => { self.apply(program[pc].clone()); pc += 1; }
            }
        }
    }

    fn apply(&mut self, brick: Brick) {
        match brick {
            Brick::SetVariable { name, value } => { self.global_variables.insert(name, value); }
            Brick::StartScene(id) => self.pending_transition = Some((id, SceneTransition::StartScene)),
            Brick::ResumeScene(id) => self.pending_transition = Some((id, SceneTransition::ResumeScene)),
            Brick::SetX(x) => { for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.x = x; } }
            Brick::SetY(y) => { for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.y = y; } }
            Brick::ChangeX(dx) => { for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.x += dx; } }
            Brick::ChangeY(dy) => { for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.y += dy; } }
            Brick::SetGravity(g) => self.gravity = g,
            Brick::PocketUp { id, arguments, .. } => self.apply_pocket_up(&id, &arguments),
            Brick::Bounce => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.velocity_y = s.velocity_y.abs(); },
            // Timing and speech UI are handled by the frame/UI layer.
            Brick::Wait(_) | Brick::Say(_) => {}
        }
    }

    fn apply_pocket_up(&mut self, id: &str, arguments: &[String]) {
        let number = |index: usize, default: f32| arguments.get(index).and_then(|value| value.trim().parse::<f32>().ok()).unwrap_or(default);
        let variable = arguments.first().filter(|name| !name.trim().is_empty()).map(|name| name.trim().to_owned()).unwrap_or_else(|| "Очки".into());
        const DEFAULT_X: f32 = 100.0;
        const DEFAULT_Y: f32 = 100.0;
        match id {
            "setPosition" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.x = number(0, DEFAULT_X); s.y = number(1, DEFAULT_Y); },
            "setPositionX" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.x = number(0, DEFAULT_X); },
            "setPositionY" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.y = number(0, DEFAULT_Y); },
            "editPositionX" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.x += number(0, 10.0); },
            "editPositionY" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.y += number(0, 10.0); },
            "goSteps" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { let radians = s.rotation.to_radians(); let steps = number(0, 10.0); s.x += radians.sin() * steps; s.y -= radians.cos() * steps; },
            "jump" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.velocity_x = number(0, 10.0); s.velocity_y = number(1, 20.0); },
            "jumpX" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.velocity_x = number(0, 10.0); },
            "jumpY" | "jumpYIf" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.velocity_y = number(0, 20.0); },
            "speedStepsToSecoond" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.velocity_x = number(0, 0.0); s.velocity_y = number(1, -10.0); },
            "setGravityAllObjects" | "setGravityScale" => self.gravity = number(0, -10.0),
            "setWeight" | "setElasticity" | "setFriction" | "setCircleHitbox" | "setQuareWHHitbox" | "setQuareHitbox" | "addBody" | "isSensor" => {},
            "setSize" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.size = number(0, 60.0); },
            "editSize" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.size = (s.size + number(0, 10.0)).max(0.0); },
            "setAlpha" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.alpha = number(0, 50.0).clamp(0.0, 100.0); },
            "editAlpha" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.alpha = (s.alpha + number(0, 25.0)).clamp(0.0, 100.0); },
            "setBrightness" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.brightness = number(0, 50.0).clamp(0.0, 100.0); },
            "editBrightness" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.brightness = (s.brightness + number(0, 25.0)).clamp(0.0, 100.0); },
            "setColor" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.tint = hue_to_rgb(number(0, 120.0)); },
            "editColor" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.tint = hue_to_rgb(number(0, 25.0)); },
            "setRotate" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.rotation = number(0, 90.0); },
            "editRotateLeft" | "rotateLeftForever" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.rotation -= number(0, 15.0); },
            "editRotateRight" | "rotateRightForever" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.rotation += number(0, 15.0); },
            "setPositionXVideo" | "setPositionXParticle" => {},
            "nextImage" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.costume = s.costume.saturating_add(1); },
            "previousImage" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.costume = s.costume.saturating_sub(1); },
            "hide" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.visible = false; },
            "show" => for s in self.sprites.iter_mut().filter(|s| !s.is_clone) { s.visible = true; },
            "setVariable" | "fbSetValue" => { self.global_variables.insert(variable, number(1, 0.0) as f64); }
            "editVariable" => *self.global_variables.entry(variable).or_insert(0.0) += number(1, 1.0) as f64,
            "deleteVariable" | "fbDelValue" => { self.global_variables.remove(&variable); }
            "setVariableToRandom" => { self.global_variables.insert(variable, number(1, 0.0) as f64); }
            "addElementArray" => self.arrays.entry(variable).or_default().push(arguments.get(1).cloned().unwrap_or_default()),
            "deleteElementArray" => { let index = number(1, 1.0).max(1.0) as usize - 1; if let Some(values) = self.arrays.get_mut(&variable) { if index < values.len() { values.remove(index); } } }
            "deleteAllElementsArray" => { self.arrays.entry(variable).or_default().clear(); }
            "pasteElementArray" => { let index = number(2, 1.0).max(1.0) as usize - 1; let value = arguments.get(1).cloned().unwrap_or_default(); let values = self.arrays.entry(variable).or_default(); values.insert(index.min(values.len()), value); }
            "replaceElementArray" => { let index = number(1, 1.0).max(1.0) as usize - 1; let value = arguments.get(2).cloned().unwrap_or_default(); if let Some(slot) = self.arrays.entry(variable).or_default().get_mut(index) { *slot = value; } }
            "deleteClone" => { if let Some(index) = self.sprites.iter().rposition(|sprite| sprite.is_clone) { self.sprites.remove(index); } }
            "deleteAllClones" => self.sprites.retain(|sprite| !sprite.is_clone),
            "clone" | "startClone" => {
                let clone_offset = self.sprites.len();
                let clones: Vec<Sprite> = self.sprites.iter().filter(|sprite| !sprite.is_clone).cloned().enumerate()
                    .map(|(index, mut sprite)| { sprite.id = format!("{}-clone-{}", sprite.id, clone_offset + index); sprite.is_clone = true; sprite }).collect();
                self.sprites.extend(clones);
            }
            // Device, media, networking and editor-widget commands need platform services;
            // they remain serializable and are not silently mapped to unrelated behavior.
            _ => {}
        }
    }

    fn deactivate(&mut self, services: &mut impl SceneServices) {
        services.stop_scene_audio(&self.definition.id);
        services.close_scene_dialog_and_keyboard(&self.definition.id);
        services.disable_scene_input_and_sensors(&self.definition.id);
        services.pause_scene_bricks(&self.definition.id);
        self.touch_enabled = false;
        self.lifecycle = SceneLifecycle::Cached;
    }

    fn unload(&mut self, services: &mut impl SceneServices) {
        self.sprites.clear(); // includes all runtime clones
        self.global_variables.clear();
        self.arrays.clear();
        services.unload_scene_assets(&self.definition.id);
        self.lifecycle = SceneLifecycle::Unloaded;
    }
}

/// Owns the active scene and cached scenes. The project definition is the source of truth
/// for a fresh StartScene; ResumeScene reuses the exact in-memory runtime instance.
pub struct SceneManager {
    definitions: HashMap<SceneId, SceneDefinition>,
    scene_order: Vec<SceneId>,
    pub cached_scenes: HashMap<SceneId, Scene>,
    pub current_scene: Option<Scene>,
}

impl SceneManager {
    pub fn new(definitions: Vec<SceneDefinition>) -> Self {
        let scene_order = definitions.iter().map(|d| d.id.clone()).collect();
        Self { definitions: definitions.into_iter().map(|d| (d.id.clone(), d)).collect(), scene_order, cached_scenes: HashMap::new(), current_scene: None }
    }

    pub fn transition(&mut self, id: &str, mode: SceneTransition, services: &mut impl SceneServices) -> Result<(), String> {
        if !self.definitions.contains_key(id) { return Err(format!("Unknown scene: {id}")); }
        if let Some(mut current) = self.current_scene.take() {
            current.deactivate(services);
            match mode {
                SceneTransition::StartScene => current.unload(services),
                SceneTransition::ResumeScene => { self.cached_scenes.insert(current.definition.id.clone(), current); }
            }
        }
        let (mut target, resumed) = match mode {
            SceneTransition::ResumeScene => match self.cached_scenes.remove(id) {
                Some(scene) => (scene, true),
                None => (Scene::load(self.definitions[id].clone()), false),
            },
            SceneTransition::StartScene => {
                if let Some(mut stale) = self.cached_scenes.remove(id) { stale.unload(services); }
                (Scene::load(self.definitions[id].clone()), false)
            }
        };
        target.activate(resumed);
        self.current_scene = Some(target);
        Ok(())
    }

    /// Remove a cached scene so the next entry reconstructs it from its definition.
    pub fn add_scene(&mut self, definition: SceneDefinition) -> Result<(), String> {
        if self.definitions.contains_key(&definition.id) { return Err(format!("Scene already exists: {}", definition.id)); }
        self.scene_order.push(definition.id.clone());
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn scene_definitions(&self) -> impl Iterator<Item = &SceneDefinition> { self.scene_order.iter().filter_map(|id| self.definitions.get(id)) }

    pub fn update_sprite(&mut self, scene_id: &str, sprite_id: &str, update: impl FnOnce(&mut Sprite)) -> Result<(), String> {
        let definition = self.definitions.get_mut(scene_id).ok_or_else(|| format!("Unknown scene: {scene_id}"))?;
        let sprite = definition.sprites.iter_mut().find(|sprite| sprite.id == sprite_id)
            .ok_or_else(|| format!("Unknown object: {sprite_id}"))?;
        update(sprite);
        if let Some(active) = self.current_scene.as_mut().filter(|scene| scene.definition.id == scene_id) {
            if let Some(runtime_sprite) = active.sprites.iter_mut().find(|sprite| sprite.id == sprite_id) {
                *runtime_sprite = sprite.clone();
            }
            active.definition = definition.clone();
        }
        Ok(())
    }

    pub fn add_sprite(&mut self, scene_id: &str, sprite: Sprite) -> Result<(), String> {
        let definition = self.definitions.get_mut(scene_id).ok_or_else(|| format!("Unknown scene: {scene_id}"))?;
        if definition.sprites.iter().any(|existing| existing.id == sprite.id) {
            return Err(format!("Object already exists: {}", sprite.id));
        }
        definition.sprites.push(sprite.clone());
        if let Some(active) = self.current_scene.as_mut().filter(|scene| scene.definition.id == scene_id) {
            active.sprites.push(sprite);
            active.definition = definition.clone();
        }
        Ok(())
    }

    pub fn remove_sprite(&mut self, scene_id: &str, sprite_id: &str) -> Result<(), String> {
        let definition = self.definitions.get_mut(scene_id).ok_or_else(|| format!("Unknown scene: {scene_id}"))?;
        if definition.sprites.len() <= 1 { return Err("Сцене нужен хотя бы один объект".into()); }
        let before = definition.sprites.len();
        definition.sprites.retain(|sprite| sprite.id != sprite_id);
        if before == definition.sprites.len() { return Err(format!("Unknown object: {sprite_id}")); }
        if let Some(active) = self.current_scene.as_mut().filter(|scene| scene.definition.id == scene_id) {
            active.sprites.retain(|sprite| sprite.id != sprite_id);
            active.definition = definition.clone();
        }
        Ok(())
    }

    pub fn add_brick(&mut self, id: &str, event: Event, brick: Brick) -> Result<(), String> {
        let definition = self.definitions.get_mut(id).ok_or_else(|| format!("Unknown scene: {id}"))?;
        if let Some(script) = definition.scripts.iter_mut().find(|s| s.event == event) { script.bricks.push(brick); }
        else { definition.scripts.push(Script { event, bricks: vec![brick] }); }
        if let Some(active) = self.current_scene.as_mut().filter(|s| s.definition.id == id) {
            active.definition = definition.clone();
        }
        Ok(())
    }

    /// Export portable project data containing scenes, sprites, event hooks and bricks.
    pub fn export_project_json(&self) -> Result<String, String> {
        let ordered: Vec<_> = self.scene_order.iter().filter_map(|id| self.definitions.get(id).cloned()).collect();
        serde_json::to_string_pretty(&ordered).map_err(|e| e.to_string())
    }

    /// Validate and replace project data, unloading active/cached runtime instances first.
    pub fn import_project_json(&mut self, json: &str, services: &mut impl SceneServices) -> Result<(), String> {
        let definitions: Vec<SceneDefinition> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if definitions.is_empty() { return Err("Проект должен содержать хотя бы одну сцену".into()); }
        let mut next = HashMap::new();
        let mut order = Vec::with_capacity(definitions.len());
        for definition in definitions {
            if definition.id.trim().is_empty() || next.contains_key(&definition.id) { return Err("Пустой или повторяющийся ID сцены".into()); }
            order.push(definition.id.clone());
            next.insert(definition.id.clone(), definition);
        }
        if let Some(mut scene) = self.current_scene.take() { scene.unload(services); }
        for (_, mut scene) in self.cached_scenes.drain() { scene.unload(services); }
        let first_id = order[0].clone();
        self.definitions = next;
        self.scene_order = order;
        self.transition(&first_id, SceneTransition::StartScene, services)
    }

    pub fn reset_scene_cache(&mut self, id: &str, services: &mut impl SceneServices) -> bool {
        if let Some(mut scene) = self.cached_scenes.remove(id) { scene.unload(services); true } else { false }
    }

    pub fn tick(&mut self, dt: f64, services: &mut impl SceneServices) {
        let Some(scene) = self.current_scene.as_mut() else { return };
        if scene.lifecycle != SceneLifecycle::Active { return; }
        scene.elapsed_seconds += dt.max(0.0);
        // Gravity is integrated only for active scenes; cached scenes are frozen.
        let step = dt.clamp(0.0, 0.1) as f32;
        for sprite in scene.sprites.iter_mut().filter(|s| !s.is_clone && s.visible) {
            sprite.velocity_y += scene.gravity * step;
            sprite.x += sprite.velocity_x * step;
            sprite.y += sprite.velocity_y * step;
            if sprite.x.abs() > 240.0 { sprite.x = sprite.x.clamp(-240.0, 240.0); sprite.velocity_x = -sprite.velocity_x; }
            if sprite.y.abs() > 135.0 { sprite.y = sprite.y.clamp(-135.0, 135.0); sprite.velocity_y = -sprite.velocity_y; }
        }
        scene.run_event(Event::Always);
        let pending = scene.pending_transition.take();
        if let Some((id, mode)) = pending { let _ = self.transition(&id, mode, services); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sprite() -> Sprite { Sprite { id: "cat".into(), x: 0.0, y: 0.0, velocity_x: 0.0, velocity_y: 0.0, size: 100.0, rotation: 0.0, alpha: 100.0, brightness: 100.0, tint: [145, 201, 77], costume: 0, visible: true, is_clone: false, local_variables: HashMap::new() } }
    fn manager() -> SceneManager {
        let scene = SceneDefinition { id: "game".into(), name: "Игра".into(), sprites: vec![sprite()], scripts: vec![Script { event: Event::Start, bricks: vec![Brick::SetVariable { name: "score".into(), value: 0.0 }] }, Script { event: Event::Resume, bricks: vec![Brick::ChangeX(1.0)] }] };
        SceneManager::new(vec![scene])
    }
    #[test]
    fn pocket_up_motion_appearance_and_data_blocks_execute() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        let scene = m.current_scene.as_mut().unwrap();
        scene.apply(Brick::PocketUp { id: "setPosition".into(), title: "Set position".into(), arguments: vec!["12".into(), "34".into()] });
        assert_eq!((scene.sprites[0].x, scene.sprites[0].y), (12.0, 34.0));
        scene.apply(Brick::PocketUp { id: "hide".into(), title: "Hide".into(), arguments: vec![] });
        assert!(!scene.sprites[0].visible);
        scene.apply(Brick::PocketUp { id: "show".into(), title: "Show".into(), arguments: vec![] });
        assert!(scene.sprites[0].visible);
        scene.apply(Brick::PocketUp { id: "setVariable".into(), title: "Set variable".into(), arguments: vec!["score".into(), "5".into()] });
        scene.apply(Brick::PocketUp { id: "editVariable".into(), title: "Change variable".into(), arguments: vec!["score".into(), "3".into()] });
        assert_eq!(scene.global_variables["score"], 8.0);
    }

    #[test]
    fn pocket_up_repeat_and_condition_blocks_run_nested_commands() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        let scene = m.current_scene.as_mut().unwrap();
        let block = |id: &str, arguments: Vec<String>| Brick::PocketUp { id: id.into(), title: id.into(), arguments };
        scene.execute_program(&[
            block("repeat", vec!["3".into()]),
            block("editPositionX", vec!["2".into()]),
            block("endRepeat", vec![]),
            block("if", vec!["4".into(), ">".into(), "2".into()]),
            block("editPositionY", vec!["5".into()]),
            block("endIf", vec![]),
        ], 0);
        assert_eq!(scene.sprites[0].x, 6.0);
        assert_eq!(scene.sprites[0].y, 5.0);
    }

    #[test]
    fn pocket_up_array_blocks_mutate_lists() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        let scene = m.current_scene.as_mut().unwrap();
        let block = |id: &str, arguments: Vec<String>| Brick::PocketUp { id: id.into(), title: id.into(), arguments };
        scene.apply(block("addElementArray", vec!["items".into(), "alpha".into()]));
        scene.apply(block("addElementArray", vec!["items".into(), "beta".into()]));
        scene.apply(block("replaceElementArray", vec!["items".into(), "1".into(), "first".into()]));
        assert_eq!(scene.arrays["items"], vec!["first", "beta"]);
    }

    #[test]
    fn object_manager_updates_definition_and_live_scene() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        let extra = Sprite { id: "star".into(), ..sprite() };
        m.add_sprite("game", extra).unwrap();
        assert_eq!(m.current_scene.as_ref().unwrap().sprites.len(), 2);
        assert!(m.add_sprite("game", sprite()).is_err());
        m.remove_sprite("game", "star").unwrap();
        assert_eq!(m.current_scene.as_ref().unwrap().sprites.len(), 1);
        assert!(m.remove_sprite("game", "cat").is_err());
    }

    #[test]
    fn resume_preserves_state_and_fires_resume_not_start() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::ResumeScene, &mut svc).unwrap();
        let s = m.current_scene.as_mut().unwrap(); s.sprites[0].x = 42.0; s.sprites[0].local_variables.insert("lives".into(), 2.0);
        m.transition("game", SceneTransition::ResumeScene, &mut svc).unwrap();
        let s = m.current_scene.as_ref().unwrap();
        assert_eq!(s.sprites[0].x, 43.0); assert_eq!(s.sprites[0].local_variables["lives"], 2.0);
        assert_eq!(s.start_events, 1); assert_eq!(s.resume_events, 1);
    }
    #[test]
    fn project_json_round_trip_preserves_bricks_and_sprites() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        m.add_brick("game", Event::Start, Brick::PocketUp { id: "setPosition".into(), title: "Set position".into(), arguments: vec!["12".into(), "34".into()] }).unwrap();
        let exported = m.export_project_json().unwrap();
        let mut restored = SceneManager::new(vec![SceneDefinition { id: "placeholder".into(), name: "old".into(), sprites: vec![sprite()], scripts: vec![] }]);
        restored.import_project_json(&exported, &mut svc).unwrap();
        let restored_game = restored.scene_definitions().find(|definition| definition.id == "game").unwrap();
        assert_eq!(restored_game.scripts[0].bricks[1], Brick::PocketUp {
            id: "setPosition".into(), title: "Set position".into(), arguments: vec!["12".into(), "34".into()],
        });
    }

    #[test]
    fn scene_transition_bricks_switch_scenes_at_runtime() {
        let mut first = SceneDefinition { id: "menu".into(), name: "Menu".into(), sprites: vec![sprite()], scripts: vec![Script { event: Event::Start, bricks: vec![Brick::StartScene("game".into())] }] };
        let second = SceneDefinition { id: "game".into(), name: "Game".into(), sprites: vec![sprite()], scripts: vec![] };
        first.sprites[0].id = "menu sprite".into();
        let mut m = SceneManager::new(vec![first, second]); let mut svc = NoopServices;
        m.transition("menu", SceneTransition::StartScene, &mut svc).unwrap();
        m.tick(0.016, &mut svc);
        assert_eq!(m.current_scene.as_ref().unwrap().definition.id, "game");
    }

    #[test]
    fn start_scene_resets_cached_runtime() {
        let mut m = manager(); let mut svc = NoopServices;
        m.transition("game", SceneTransition::ResumeScene, &mut svc).unwrap();
        m.current_scene.as_mut().unwrap().sprites[0].x = 90.0;
        m.transition("game", SceneTransition::StartScene, &mut svc).unwrap();
        assert_eq!(m.current_scene.as_ref().unwrap().sprites[0].x, 0.0);
        assert_eq!(m.current_scene.as_ref().unwrap().start_events, 1);
    }
}
