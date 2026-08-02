use glam::Vec2;

use crate::constants::{
    FILTH_ATTACK_COOLDOWN, FILTH_ATTACK_DAMAGE, FILTH_ATTACK_RANGE, FILTH_HEALTH, FILTH_SIZE,
    FILTH_SPEED,
};
use crate::game::level::{CollisionGeometry, Solid};
use crate::game::portal::Portal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Filth,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub spawn_pos: Vec2,
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    pub health: f32,
    pub spawn_id: u16,
    pub spawn_wave: u16,
    pub active: bool,
    pub spawned: bool,
    pub attack_cooldown: f32,
    pub hurt_flash: f32,
    pub on_ground: bool,
}

#[derive(Clone, Copy)]
pub struct EnemyUpdateContext<'a> {
    pub dt: f32,
    pub target_pos: Vec2,
    pub can_attack: bool,
    pub collision: CollisionGeometry<'a>,
    pub portals: &'a [Portal],
    pub speed_multiplier: f32,
    pub damage_multiplier: f32,
}

impl Enemy {
    pub fn filth(x: f32, y: f32) -> Self {
        Self {
            kind: EnemyKind::Filth,
            spawn_pos: Vec2::new(x, y),
            pos: Vec2::new(x, y),
            prev_pos: Vec2::new(x, y),
            vel: Vec2::ZERO,
            health: FILTH_HEALTH,
            spawn_id: 0,
            spawn_wave: 0,
            active: true,
            spawned: true,
            attack_cooldown: 0.0,
            hurt_flash: 0.0,
            on_ground: false,
        }
    }

    pub fn filth_spawn(x: f32, y: f32, spawn_id: u16, spawn_wave: u16) -> Self {
        let mut enemy = Self::filth(x, y);

        enemy.spawn_id = spawn_id;
        enemy.spawn_wave = spawn_wave.max(1);
        enemy.active = false;
        enemy.spawned = false;
        enemy
    }

    pub fn reset_for_level_start(&mut self) {
        self.pos = self.spawn_pos;
        self.prev_pos = self.pos;
        self.vel = Vec2::ZERO;
        self.health = FILTH_HEALTH;
        self.attack_cooldown = 0.0;
        self.hurt_flash = 0.0;
        self.on_ground = false;
        self.active = self.spawn_wave == 0;
        self.spawned = self.spawn_wave == 0;
    }

    pub fn activate_spawn(&mut self) {
        self.pos = self.spawn_pos;
        self.prev_pos = self.pos;
        self.vel = Vec2::ZERO;
        self.health = FILTH_HEALTH;
        self.attack_cooldown = 0.0;
        self.hurt_flash = 0.0;
        self.on_ground = false;
        self.active = true;
        self.spawned = true;
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn is_active(&self) -> bool {
        self.active && self.is_alive()
    }

    pub fn mark_dead(&mut self) {
        self.active = false;
    }

    pub fn size(&self) -> Vec2 {
        match self.kind {
            EnemyKind::Filth => Vec2::new(FILTH_SIZE.0, FILTH_SIZE.1),
        }
    }

    pub fn half_size(&self) -> Vec2 {
        self.size() / 2.0
    }

    pub fn spawn_solid(&self) -> Solid {
        self.solid_at(self.spawn_pos)
    }

    pub fn solid(&self) -> Solid {
        self.solid_at(self.pos)
    }

    fn solid_at(&self, pos: Vec2) -> Solid {
        let size = self.size();

        Solid::new(
            pos.x - size.x / 2.0,
            pos.y - size.y / 2.0,
            size.x,
            size.y,
            false,
        )
    }

    pub fn update(&mut self, context: EnemyUpdateContext<'_>) -> Option<f32> {
        let dt = context.dt;

        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);

        match self.kind {
            EnemyKind::Filth => self.update_filth(context),
        }
    }

    pub fn damage(&mut self, amount: f32) -> bool {
        if amount <= 0.0 {
            return false;
        }

        self.health -= amount;
        self.hurt_flash = 0.12;

        self.health <= 0.0
    }

    fn update_filth(&mut self, context: EnemyUpdateContext<'_>) -> Option<f32> {
        self.prev_pos = self.pos;

        let to_target = context.target_pos - self.pos;
        let dir_x = if to_target.x.abs() > 2.0 {
            to_target.x.signum()
        } else {
            0.0
        };

        self.vel.x = dir_x * FILTH_SPEED * context.speed_multiplier.max(0.0);
        self.vel.y += crate::constants::GRAVITY * context.dt;
        self.pos += self.vel * context.dt;
        let half_size = self.half_size();
        self.on_ground = context.collision.resolve_actor_body_with_portals(
            &mut self.pos,
            half_size,
            &mut self.vel,
            context.portals,
        );

        if context.can_attack
            && to_target.length() <= FILTH_ATTACK_RANGE
            && self.attack_cooldown <= 0.0
        {
            self.attack_cooldown = FILTH_ATTACK_COOLDOWN;
            return Some(FILTH_ATTACK_DAMAGE * context.damage_multiplier.max(0.0));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::level::Level;

    #[test]
    fn filth_attack_respects_cooldown() {
        let level = Level {
            solids: vec![Solid::new(0.0, 200.0, 420.0, 24.0, false)],
            ..Level::empty()
        };
        let portals = [];
        let collision = CollisionGeometry::new(&level.solids, &level.doors);
        let mut enemy = Enemy::filth(100.0, 171.0);
        let context = EnemyUpdateContext {
            dt: 1.0 / 60.0,
            target_pos: Vec2::new(120.0, 171.0),
            can_attack: true,
            collision,
            portals: &portals,
            speed_multiplier: 1.0,
            damage_multiplier: 1.0,
        };

        let first_attack = enemy.update(context);
        let second_attack = enemy.update(context);

        assert!(first_attack.is_some_and(|damage| damage > 0.0));
        assert!(second_attack.is_none());
        assert!(enemy.attack_cooldown > 0.0);
    }
}
