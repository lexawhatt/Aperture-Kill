use glam::Vec2;

use crate::constants::{
    FILTH_ATTACK_COOLDOWN, FILTH_ATTACK_DAMAGE, FILTH_ATTACK_RANGE, FILTH_HEALTH, FILTH_SIZE,
    FILTH_SPEED,
};
use crate::game::level::{CollisionGeometry, Solid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Filth,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub health: f32,
    pub attack_cooldown: f32,
    pub hurt_flash: f32,
    pub on_ground: bool,
}

impl Enemy {
    pub fn filth(x: f32, y: f32) -> Self {
        Self {
            kind: EnemyKind::Filth,
            pos: Vec2::new(x, y),
            vel: Vec2::ZERO,
            health: FILTH_HEALTH,
            attack_cooldown: 0.0,
            hurt_flash: 0.0,
            on_ground: false,
        }
    }

    pub fn size(&self) -> Vec2 {
        match self.kind {
            EnemyKind::Filth => Vec2::new(FILTH_SIZE.0, FILTH_SIZE.1),
        }
    }

    pub fn half_size(&self) -> Vec2 {
        self.size() / 2.0
    }

    pub fn solid(&self) -> Solid {
        let size = self.size();

        Solid::new(
            self.pos.x - size.x / 2.0,
            self.pos.y - size.y / 2.0,
            size.x,
            size.y,
            false,
        )
    }

    pub fn update(
        &mut self,
        dt: f32,
        player_pos: Vec2,
        collision: CollisionGeometry<'_>,
    ) -> Option<f32> {
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);

        match self.kind {
            EnemyKind::Filth => self.update_filth(dt, player_pos, collision),
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

    fn update_filth(
        &mut self,
        dt: f32,
        player_pos: Vec2,
        collision: CollisionGeometry<'_>,
    ) -> Option<f32> {
        let to_player = player_pos - self.pos;
        let dir_x = to_player.x.signum();

        self.vel.x = dir_x * FILTH_SPEED;
        self.vel.y += crate::constants::GRAVITY * dt;
        self.pos += self.vel * dt;
        let half_size = self.half_size();
        self.on_ground = collision.resolve_actor_body(&mut self.pos, half_size, &mut self.vel);

        if to_player.length() <= FILTH_ATTACK_RANGE && self.attack_cooldown <= 0.0 {
            self.attack_cooldown = FILTH_ATTACK_COOLDOWN;
            return Some(FILTH_ATTACK_DAMAGE);
        }

        None
    }
}
