//! Scene lifecycle and cache for the PocketCat runtime.
use std::collections::HashMap;

pub type SceneId = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneTransition { StartScene, ResumeScene }

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub costume: usize,
    pub visible: bool,
    pub is_clone: bool,
    pub local_variables: HashMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq)]
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { Start, Resume, Touch, Always }

#[derive(Clone, Debug, PartialEq)]
pub struct Script { pub event: Event, pub bricks: Vec<Brick> }

#[derive(Clone, Debug, PartialEq)]
pub struct SceneDefinition {
    pub id: SceneId,
    pub name: String,
    pub sprites: Vec<Sprite>,
    pub scripts: Vec<Script>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneLifecycle { Loaded, Active, Cached, Unloaded }

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
            global_variables: HashMap::new(), lifecycle: SceneLifecycle::Loaded,
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
        let bricks: Vec<Brick> = self.definition.scripts.iter()
            .filter(|s| s.event == event).flat_map(|s| s.bricks.clone()).collect();
        for brick in bricks { self.apply(brick); }
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
            // Detailed physics integration belongs to the frame tick.
            Brick::Bounce | Brick::Wait(_) | Brick::Say(_) => {}
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
        services.unload_scene_assets(&self.definition.id);
        self.lifecycle = SceneLifecycle::Unloaded;
    }
}

/// Owns the active scene and cached scenes. The project definition is the source of truth
/// for a fresh StartScene; ResumeScene reuses the exact in-memory runtime instance.
pub struct SceneManager {
    definitions: HashMap<SceneId, SceneDefinition>,
    pub cached_scenes: HashMap<SceneId, Scene>,
    pub current_scene: Option<Scene>,
}

impl SceneManager {
    pub fn new(definitions: Vec<SceneDefinition>) -> Self {
        Self { definitions: definitions.into_iter().map(|d| (d.id.clone(), d)).collect(), cached_scenes: HashMap::new(), current_scene: None }
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
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn scene_definitions(&self) -> impl Iterator<Item = &SceneDefinition> { self.definitions.values() }

    pub fn add_brick(&mut self, id: &str, event: Event, brick: Brick) -> Result<(), String> {
        let definition = self.definitions.get_mut(id).ok_or_else(|| format!("Unknown scene: {id}"))?;
        if let Some(script) = definition.scripts.iter_mut().find(|s| s.event == event) { script.bricks.push(brick); }
        else { definition.scripts.push(Script { event, bricks: vec![brick] }); }
        if let Some(active) = self.current_scene.as_mut().filter(|s| s.definition.id == id) {
            active.definition = definition.clone();
        }
        Ok(())
    }

    pub fn reset_scene_cache(&mut self, id: &str, services: &mut impl SceneServices) -> bool {
        if let Some(mut scene) = self.cached_scenes.remove(id) { scene.unload(services); true } else { false }
    }

    pub fn tick(&mut self, dt: f64, services: &mut impl SceneServices) {
        let Some(scene) = self.current_scene.as_mut() else { return };
        if scene.lifecycle != SceneLifecycle::Active { return; }
        scene.elapsed_seconds += dt.max(0.0);
        // Gravity is integrated only for active scenes; cached scenes are frozen.
        for sprite in scene.sprites.iter_mut().filter(|s| !s.is_clone && s.visible) {
            sprite.y += scene.gravity * dt as f32 * dt as f32 * 0.5;
        }
        scene.run_event(Event::Always);
        let pending = scene.pending_transition.take();
        if let Some((id, mode)) = pending { let _ = self.transition(&id, mode, services); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sprite() -> Sprite { Sprite { id: "cat".into(), x: 0.0, y: 0.0, costume: 0, visible: true, is_clone: false, local_variables: HashMap::new() } }
    fn manager() -> SceneManager {
        let scene = SceneDefinition { id: "game".into(), name: "Игра".into(), sprites: vec![sprite()], scripts: vec![Script { event: Event::Start, bricks: vec![Brick::SetVariable { name: "score".into(), value: 0.0 }] }, Script { event: Event::Resume, bricks: vec![Brick::ChangeX(1.0)] }] };
        SceneManager::new(vec![scene])
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
