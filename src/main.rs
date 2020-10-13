use bevy::prelude::*;
use rand::distributions::Uniform;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use std::mem;

const BOARD_SIZE: u32 = 16;
const SIZE: f32 = 30.;
const INIT_LENGTH: usize = 4;
const SPEED: u64 = 150;
const DEATH_TIME: u64 = 600;
const FOOD_BREAK: u64 = 100;
const FORGIVENESS_BREAK: u64 = 100;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Properties, Default)]
struct Pos {
    x: u32,
    y: u32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Eq, PartialEq)]
enum SnakeState {
    Alive,
    Dead,
    Forgive,
    Pause(Box<SnakeState>),
}

struct GameState {
    head: usize,
    body: Vec<Entity>,
    body_pos_set: HashSet<Pos>,
    dir: VecDeque<Dir>,
    body_material: Handle<ColorMaterial>,
    snake_state: SnakeState,
}

impl GameState {
    #[inline]
    pub fn score(&self) -> usize {
        self.body.len() - INIT_LENGTH
    }
}

impl Pos {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn to_world(&self, z: f32) -> Vec3 {
        Vec3::new(
            (self.x as f32 - BOARD_SIZE as f32 / 2.0) * SIZE,
            (self.y as f32 - BOARD_SIZE as f32 / 2.0) * SIZE,
            z,
        )
    }

    pub fn update(&mut self, dir: Dir) -> bool {
        match dir {
            Dir::Right => {
                if self.x == BOARD_SIZE - 1 {
                    true
                } else {
                    self.x += 1;
                    false
                }
            }
            Dir::Up => {
                if self.y == BOARD_SIZE - 1 {
                    true
                } else {
                    self.y += 1;
                    false
                }
            }
            Dir::Left => {
                if 0 < self.x {
                    self.x -= 1;
                    false
                } else {
                    true
                }
            }
            Dir::Down => {
                if 0 < self.y {
                    self.y -= 1;
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        let distr = Uniform::from(0..BOARD_SIZE);
        self.x = rng.sample(distr);
        self.y = rng.sample(distr)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Property)]
enum TagType {
    ScoreText,
    Food,
}

impl Default for TagType {
    fn default() -> Self {
        Self::Food
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Properties)]
struct Tag(TagType);

impl Tag {
    pub const fn score_text() -> Self {
        Self(TagType::ScoreText)
    }

    pub const fn food() -> Self {
        Self(TagType::Food)
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state: ResMut<GameState>,
) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    commands
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                value: "Score:".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 40.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        })
        .with(Tag::score_text());

    // Create materials
    let bg_material = materials.add(Color::BLACK.into());
    let food_material = materials.add(Color::RED.into());
    state.body_material = materials.add(Color::WHITE.into());

    // Spawn body
    for i in 0..INIT_LENGTH {
        let pos = Pos::new(i as u32 + 1, BOARD_SIZE / 2);
        commands
            .spawn(SpriteComponents {
                material: state.body_material,
                sprite: Sprite::new(Vec2::splat(SIZE)),
                transform: Transform::from_translation(pos.to_world(2.0)),
                ..Default::default()
            })
            .with(pos);
        state.body.push(commands.current_entity().unwrap());
        state.body_pos_set.insert(pos);
    }

    // Spawn food
    let pos = Pos::new(BOARD_SIZE * 3 / 4, BOARD_SIZE / 2);
    commands
        .spawn(SpriteComponents {
            material: food_material,
            sprite: Sprite::new(Vec2::splat(SIZE)),
            transform: Transform::from_translation(pos.to_world(1.0)),
            ..Default::default()
        })
        .with(pos)
        .with(Tag::food());

    // Spawn background
    commands.spawn(SpriteComponents {
        material: bg_material,
        sprite: Sprite::new(Vec2::new(
            SIZE * BOARD_SIZE as f32,
            SIZE * BOARD_SIZE as f32,
        )),
        transform: Transform::from_translation(Vec2::splat(-SIZE / 2.0).extend(0.0)),
        ..Default::default()
    });
}

fn update(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    input: Res<Input<KeyCode>>,
    mut timer: ResMut<UpdateTimer>,
    time: Res<Time>,
    body_query: Query<(&mut Transform, &mut Pos)>,
    mut food_query: Query<(&mut Transform, &mut Pos, &Tag)>,
    mut text_query: Query<(&mut Text, &Tag)>,
) {
    if let SnakeState::Pause(ref prev) = state.snake_state {
        if input.just_pressed(KeyCode::Space) {
            state.snake_state = (**prev).clone();
        } else {
            return;
        }
    } else if input.just_pressed(KeyCode::Space) {
        let prev = mem::replace(&mut state.snake_state, SnakeState::Dead);
        state.snake_state = SnakeState::Pause(Box::new(prev));
        return;
    }
    timer.0.tick(time.delta_seconds);

    if matches!(state.snake_state, SnakeState::Alive | SnakeState::Forgive) {
        // Push to the direction buffer
        let prev_dir = *state.dir.back().unwrap();
        let mut new_dir = None;
        match prev_dir {
            Dir::Up | Dir::Down => {
                let mut dx = 0;
                if input.just_pressed(KeyCode::Left) || input.just_pressed(KeyCode::A) {
                    dx -= 1;
                }
                if input.just_pressed(KeyCode::Right) || input.just_pressed(KeyCode::D) {
                    dx += 1;
                }

                if dx == -1 {
                    new_dir = Some(Dir::Left);
                } else if dx == 1 {
                    new_dir = Some(Dir::Right);
                }
            }
            Dir::Right | Dir::Left => {
                let mut dy = 0;
                if input.just_pressed(KeyCode::Down) || input.just_pressed(KeyCode::S) {
                    dy -= 1;
                }
                if input.just_pressed(KeyCode::Up) || input.just_pressed(KeyCode::W) {
                    dy += 1;
                }

                if dy == -1 {
                    new_dir = Some(Dir::Down);
                } else if dy == 1 {
                    new_dir = Some(Dir::Up);
                }
            }
        }
        if let Some(new_dir) = new_dir {
            state.dir.push_back(new_dir);
            if state.snake_state == SnakeState::Forgive {
                timer.0.reset();
                timer.0.just_finished = true;
                timer.0.finished = true;
            }
        }
    }

    if timer.0.finished {
        timer.0.duration = Duration::from_millis(SPEED).as_secs_f32();
        if 1 < state.dir.len() {
            state.dir.pop_front();
        }
        let dir = *state.dir.front().unwrap();
        let prev_head = state.body[state.head];
        state.head = (state.head + 1) % state.body.len();
        let head = state.body[state.head];

        let mut head_pos = *body_query.get::<Pos>(prev_head).unwrap();
        if head_pos.update(dir)
            || (!state.body_pos_set.insert(head_pos)
                && *body_query.get::<Pos>(head).unwrap() != head_pos)
        {
            match state.snake_state {
                SnakeState::Alive => {
                    timer.0.duration = Duration::from_millis(FORGIVENESS_BREAK).as_secs_f32();
                    timer.0.reset();
                    state.snake_state = SnakeState::Forgive;
                    state.head = (state.body.len() + state.head - 1) % state.body.len();
                    return;
                }
                SnakeState::Forgive => {
                    die(commands, state, timer, body_query, food_query);
                    return;
                }
                _ => unreachable!(),
            }
        }

        for (mut text, tag) in &mut text_query.iter() {
            if *tag == Tag::score_text() {
                text.value = format!("Score: {}", state.score()); // .into();
            }
        }

        let mut ate = false;
        for (mut food_transform, mut food_pos, tag) in &mut food_query.iter() {
            if *tag == Tag::food() && *food_pos == head_pos {
                loop {
                    food_pos.randomize();
                    if !state.body_pos_set.contains(&food_pos) {
                        break;
                    }
                }
                food_transform.set_translation(food_pos.to_world(1.0));
                ate = true;
            }
        }
        if ate {
            timer.0.duration = Duration::from_millis(FOOD_BREAK).as_secs_f32();
            let tail = (state.head + 1) % state.body.len();
            let pos = *body_query.get::<Pos>(state.body[tail]).unwrap();
            commands
                .spawn(SpriteComponents {
                    material: state.body_material,
                    transform: Transform::from_translation(pos.to_world(2.0)),
                    sprite: Sprite::new(Vec2::splat(SIZE)),
                    ..Default::default()
                })
                .with(pos);
            state.body.insert(tail, commands.current_entity().unwrap());
        }

        // Update
        let mut head_pos_ref = body_query.get_mut::<Pos>(head).unwrap();
        state.body_pos_set.remove(&head_pos_ref);
        *head_pos_ref = head_pos;
        // mem::drop(head_pos_ref);

        body_query
            .get_mut::<Transform>(head)
            .unwrap()
            .set_translation(head_pos.to_world(2.0));

        state.snake_state = SnakeState::Alive;
    }
}

fn die(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    mut timer: ResMut<UpdateTimer>,
    body_query: Query<(&mut Transform, &mut Pos)>,
    mut food_query: Query<(&mut Transform, &mut Pos, &Tag)>,
) {
    println!("Score: {}", state.score());
    timer.0.reset();
    timer.0.duration = Duration::from_millis(DEATH_TIME).as_secs_f32();

    state.head = INIT_LENGTH - 1;
    state.body_pos_set.clear();
    state.dir.clear();
    state.dir.push_back(Dir::Right);
    state.snake_state = SnakeState::Dead;
    for _ in INIT_LENGTH..state.body.len() {
        commands.despawn(state.body.pop().unwrap());
    }
    for i in 0..INIT_LENGTH {
        let pos = Pos::new(i as u32 + 1, BOARD_SIZE / 2);
        state.body_pos_set.insert(pos);
        let entity = state.body[i];
        body_query
            .get_mut::<Transform>(entity)
            .unwrap()
            .set_translation(pos.to_world(2.0));
        *body_query.get_mut::<Pos>(entity).unwrap() = pos;
    }

    let pos = Pos::new(BOARD_SIZE * 3 / 4, BOARD_SIZE / 2);
    for (mut food_transform, mut food_pos, tag) in &mut food_query.iter() {
        if *tag == Tag::food() {
            food_transform.set_translation(pos.to_world(1.0));
            *food_pos = pos;
        }
    }
}

struct UpdateTimer(Timer);

fn main() {
    App::build()
        .add_default_plugins()
        .add_resource(GameState {
            head: INIT_LENGTH - 1,
            body: Vec::with_capacity(INIT_LENGTH),
            body_pos_set: HashSet::new(),
            dir: vec![Dir::Right].into(),
            body_material: Handle::new(),
            snake_state: SnakeState::Dead,
        })
        .add_resource(UpdateTimer(Timer::new(
            Duration::from_millis(DEATH_TIME),
            true,
        )))
        .register_component::<Pos>()
        .register_component::<Tag>()
        .add_startup_system(setup.system())
        .add_system(update.system())
        .run();
}
