use std::time::Duration;

use bevy::{
	app::{App, PluginGroup, PostUpdate, Startup, Update},
	core_pipeline::{core_2d::Camera2dBundle, tonemapping::Tonemapping},
	ecs::{
		component::Component,
		entity::Entity,
		event::{Event, EventReader, EventWriter},
		query::With,
		schedule::IntoSystemConfigs,
		system::{Commands, Local, Query, Res, ResMut, Resource},
	},
	input::{keyboard::KeyCode, ButtonInput},
	math::Vec3,
	prelude::{Deref, DerefMut},
	render::{
		camera::{Camera, ClearColor},
		color::Color,
	},
	sprite::{Sprite, SpriteBundle},
	time::common_conditions::on_timer,
	transform::components::Transform,
	utils::default,
	window::{PrimaryWindow, Window, WindowPlugin},
	DefaultPlugins,
};
use rand::random;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

#[derive(Component)]
struct SnakeHead {
	direction: Direction,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
	x: i32,
	y: i32,
}

#[derive(Component)]
struct Size {
	width: f32,
	height: f32,
}
impl Size {
	pub fn square(x: f32) -> Self {
		Self {
			width: x,
			height: x,
		}
	}
}

#[derive(Component)]
struct Food;

#[derive(PartialEq, Copy, Clone)]
enum Direction {
	Left,
	Up,
	Right,
	Down,
}
impl Direction {
	fn oppsite(self) -> Self {
		match self {
			Self::Left => Self::Right,
			Self::Right => Self::Left,
			Self::Up => Self::Down,
			Self::Down => Self::Up,
		}
	}
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Resource, Deref, DerefMut)]
struct SnakeSegments(Vec<Entity>);

#[derive(Default, Event)]
struct GrowthEvent;

#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);

fn setup_camera(mut commands: Commands) {
	commands.spawn((Camera2dBundle {
		camera: Camera {
			hdr: true,
			..default()
		},
		tonemapping: Tonemapping::TonyMcMapface,
		..default()
	},));
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
	*segments = SnakeSegments(vec![
		commands
			.spawn(SpriteBundle {
				sprite: Sprite {
					color: SNAKE_HEAD_COLOR,
					..default()
				},
				..default()
			})
			.insert(SnakeHead {
				direction: Direction::Up,
			})
			.insert(SnakeSegment)
			.insert(Position { x: 3, y: 3 })
			.insert(Size::square(0.8))
			.id(),
		spawn_segment(commands, Position { x: 3, y: 2 }),
	]);
}

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
	commands
		.spawn(SpriteBundle {
			sprite: Sprite {
				color: SNAKE_SEGMENT_COLOR,
				..default()
			},
			..default()
		})
		.insert(SnakeSegment)
		.insert(position)
		.insert(Size::square(0.65))
		.id()
}

fn food_spawner(mut commands: Commands) {
	commands
		.spawn(SpriteBundle {
			sprite: Sprite {
				color: FOOD_COLOR,
				..default()
			},
			..default()
		})
		.insert(Food)
		.insert(Position {
			x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
			y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
		})
		.insert(Size::square(0.8));
}

fn snake_movement_input(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut heads: Query<&mut SnakeHead>,
) {
	if let Some(mut head) = heads.iter_mut().next() {
		let dir: Direction = if keyboard_input.pressed(KeyCode::ArrowLeft) {
			Direction::Left
		} else if keyboard_input.pressed(KeyCode::ArrowDown) {
			Direction::Down
		} else if keyboard_input.pressed(KeyCode::ArrowUp) {
			Direction::Up
		} else if keyboard_input.pressed(KeyCode::ArrowRight) {
			Direction::Right
		} else {
			head.direction
		};
		if dir != head.direction.oppsite() {
			head.direction = dir;
		}
	}
}

fn snake_movement(
	segments: ResMut<SnakeSegments>,
	mut heads: Query<(Entity, &SnakeHead)>,
	mut positions: Query<&mut Position>,
	mut last_tail_position: ResMut<LastTailPosition>,
) {
	if let Some((head_entity, head)) = heads.iter_mut().next() {
		let segment_positions = segments
			.iter()
			.map(|e| *positions.get_mut(*e).unwrap())
			.collect::<Vec<Position>>();
		let mut head_pos = positions.get_mut(head_entity).unwrap();
		match &head.direction {
			Direction::Left => {
				head_pos.x -= 1;
			}
			Direction::Right => {
				head_pos.x += 1;
			}
			Direction::Up => {
				head_pos.y += 1;
			}
			Direction::Down => {
				head_pos.y -= 1;
			}
		};
		segment_positions
			.iter()
			.zip(segments.iter().skip(1))
			.for_each(|(pos, segment)| {
				*positions.get_mut(*segment).unwrap() = *pos;
			});
		*last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()))
	}
}

fn size_scaling(
	primary_query: Query<&Window, With<PrimaryWindow>>,
	mut q: Query<(&Size, &mut Transform)>,
) {
	let window = primary_query.get_single().unwrap();
	for (sprite_size, mut transform) in q.iter_mut() {
		transform.scale = Vec3::new(
			sprite_size.width / ARENA_WIDTH as f32 * window.width(),
			sprite_size.height / ARENA_HEIGHT as f32 * window.height(),
			1.0,
		)
	}
}

fn position_translation(
	primary_query: Query<&Window, With<PrimaryWindow>>,
	mut q: Query<(&Position, &mut Transform)>,
) {
	fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
		let title_size = bound_window / bound_game;
		pos / bound_game * bound_window - (bound_window / 2.) + (title_size / 2.)
	}

	let window = primary_query.get_single().unwrap();
	for (pos, mut transform) in q.iter_mut() {
		transform.translation = Vec3::new(
			convert(pos.x as f32, window.width(), ARENA_WIDTH as f32),
			convert(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
			0.0,
		);
	}
}

fn snake_eating(
	mut commands: Commands,
	mut growth_writer: EventWriter<GrowthEvent>,
	food_positions: Query<(Entity, &Position), With<Food>>,
	head_positions: Query<&Position, With<SnakeHead>>,
) {
	for head_pos in head_positions.iter() {
		for (ent, food_pos) in food_positions.iter() {
			if food_pos == head_pos {
				commands.entity(ent).despawn();
				growth_writer.send(GrowthEvent);
			}
		}
	}
}

fn snake_growth(
	commands: Commands,
	last_tail_position: Res<LastTailPosition>,
	mut segments: ResMut<SnakeSegments>,
	mut growth_reader: Local<EventReader<GrowthEvent>>,
) {
	if growth_reader.iter().next().is_some() {
		segments.push(spawn_segment(commands, last_tail_position.0.unwrap()))
	}
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				resolution: (500.0, 500.0).into(),
				title: "Block Bite".to_string(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
		.insert_resource(SnakeSegments::default())
		.insert_resource(LastTailPosition::default())
		.add_systems(Startup, setup_camera)
		.add_systems(Startup, spawn_snake)
		.add_systems(
			Update,
			(
				snake_movement_input.before(snake_movement),
				snake_movement.run_if(on_timer(Duration::from_secs_f32(0.150))),
				snake_eating,
				snake_growth,
			),
		)
		// .add_systems(Update, snake_movement_input.before(snake_movement))
		.add_systems(
			Update,
			food_spawner.run_if(on_timer(Duration::from_secs_f32(1.0))),
		)
		.add_systems(PostUpdate, (position_translation, size_scaling))
		.add_event::<GrowthEvent>()
		.run();
}
