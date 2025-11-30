use std::time::Instant;
use std::hint::black_box;

// AoS
struct Enemy {
    position: [f32; 3],
    velocity: [f32; 3],
    health:   f32,
    id:       u64,
}

// SoA
struct Enemies {
    positions:  Vec<[f32; 3]>,
    velocities: Vec<[f32; 3]>, 
    healths:    Vec<f32>, 
    ids:        Vec<u64>,  
}

impl Enemy {
    fn create_aos(count: usize) -> Vec<Enemy> {
        (0..count).map(|i| Enemy { 
            position: [i as f32, 0.0, 0.0],
            velocity: [1.0, 0.0, 0.0],
            health: 100.0,
            id: i as u64,
        }).collect()
    }

    fn update_positions_aos(enemies: &mut Vec<Enemy>) {
        for enemy in enemies {
            enemy.position[0] += enemy.velocity[0];
            enemy.position[1] += enemy.velocity[1];
            enemy.position[2] += enemy.velocity[2];
        }
    }
}

impl Enemies {
    fn create_soa(count: usize) -> Enemies {
        Enemies {
            positions:  (0..count).map(|i| [i as f32, 0.0, 0.0]).collect(),
            velocities: (0..count).map(|_| [1.0, 0.0, 0.0]).collect(),
            healths:    (0..count).map(|_| 100.0).collect(),
            ids:        (0..count).map(|i| i as u64).collect(),
        }
    }

    fn update_positions_soa(enemies: &mut Enemies) {
        for (pos, vel) in enemies.positions.iter_mut().zip(enemies.velocities.iter()) {
            pos[0] += vel[0];
            pos[1] += vel[1];
            pos[2] += vel[2];
        }
    }
}

const COUNT: usize = 1_000_000;

fn main() {
    // ============ AoS benchmark ============
    let mut aos_enemies = Enemy::create_aos(COUNT);
    
    let start = Instant::now();
 
    for _ in 0..100 {
        black_box(Enemy::update_positions_aos(&mut aos_enemies));
    }
    let aos_time = start.elapsed();
    
    println!("AoS: {:?}", aos_time);

    // ============ SoA benchmark ============
    let mut soa_enemies = Enemies::create_soa(COUNT);

    let start = Instant::now();

    for _ in 0..100 {
        black_box(Enemies::update_positions_soa(&mut soa_enemies));
    }
    let soa_time = start.elapsed();

    println!("SoA: {:?}", soa_time);
    
    // ============ Result ============
    println!("\n=== Result ===");
    let ratio = aos_time.as_nanos() as f64 / soa_time.as_nanos() as f64;
    println!("AoS / SoA = {:.2}x", ratio);
    
    if ratio > 1.0 {
        println!("SoA is {:.2} times faster than AoS", ratio);
    } else {
        println!("AoS is {:.2} times faster than SoA", 1.0 / ratio);
    }

}B is three times faster than A