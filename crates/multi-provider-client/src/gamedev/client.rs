#![allow(clippy::needless_raw_string_hashes)]

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::Result;
use directories::ProjectDirs;
use reqwest::Client;
use tokio::sync::Mutex;
use tracing::{instrument, warn};

use super::types::{
    GameDevCategory, GameDevCategoryItem, GameDevExample, GameDevMethod,
    GameDevMethodIndex, GameDevFramework, GameDevParameter,
    GameDevTechnology,
    SPRITEKIT_CORE, SPRITEKIT_ACTIONS, SPRITEKIT_PHYSICS,
    SCENEKIT_CORE, SCENEKIT_MATERIALS, SCENEKIT_ANIMATION, SCENEKIT_PHYSICS,
    REALITYKIT_CORE, REALITYKIT_COMPONENTS, REALITYKIT_AR,
    GAMEKIT_CORE, GAMEKIT_MULTIPLAYER,
    GAMECONTROLLER_CORE,
    GAMEDEV_PATTERNS, GAMEDEV_OPTIMIZATION,
};
use docs_mcp_client::cache::{DiskCache, MemoryCache};

const SPRITEKIT_URL: &str = "https://developer.apple.com/documentation/spritekit";
const SCENEKIT_URL: &str = "https://developer.apple.com/documentation/scenekit";
const REALITYKIT_URL: &str = "https://developer.apple.com/documentation/realitykit";
const GAMEKIT_URL: &str = "https://developer.apple.com/documentation/gamekit";
const GAMECONTROLLER_URL: &str = "https://developer.apple.com/documentation/gamecontroller";
const GAMEPLAYKIT_URL: &str = "https://developer.apple.com/documentation/gameplaykit";

#[derive(Debug)]
#[allow(dead_code)]
pub struct GameDevClient {
    http: Client,
    disk_cache: DiskCache,
    memory_cache: MemoryCache<String>,
    fetch_lock: Mutex<()>,
    cache_dir: PathBuf,
}

impl Default for GameDevClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GameDevClient {
    #[must_use]
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "RecordAndLearn", "multi-docs-mcp")
            .expect("unable to resolve project directories");

        let cache_dir = project_dirs.cache_dir().join("gamedev");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create GameDev cache directory");
        }

        let http = Client::builder()
            .user_agent("MultiDocsMCP/1.0")
            .timeout(StdDuration::from_secs(30))
            .gzip(true)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            disk_cache: DiskCache::new(&cache_dir),
            memory_cache: MemoryCache::new(time::Duration::hours(1)),
            fetch_lock: Mutex::new(()),
            cache_dir,
        }
    }

    /// Get available technologies (game dev frameworks)
    #[instrument(name = "gamedev_client.get_technologies", skip(self))]
    pub async fn get_technologies(&self) -> Result<Vec<GameDevTechnology>> {
        let spritekit = GameDevTechnology {
            identifier: "gamedev:spritekit".to_string(),
            title: "SpriteKit".to_string(),
            description: format!(
                "SpriteKit 2D Game Engine - {} types for sprites, actions, physics, and tile maps",
                SPRITEKIT_CORE.len() + SPRITEKIT_ACTIONS.len() + SPRITEKIT_PHYSICS.len()
            ),
            url: SPRITEKIT_URL.to_string(),
            item_count: SPRITEKIT_CORE.len() + SPRITEKIT_ACTIONS.len() + SPRITEKIT_PHYSICS.len(),
        };

        let scenekit = GameDevTechnology {
            identifier: "gamedev:scenekit".to_string(),
            title: "SceneKit".to_string(),
            description: format!(
                "SceneKit 3D Game Engine - {} types for nodes, materials, animation, and physics",
                SCENEKIT_CORE.len() + SCENEKIT_MATERIALS.len() + SCENEKIT_ANIMATION.len() + SCENEKIT_PHYSICS.len()
            ),
            url: SCENEKIT_URL.to_string(),
            item_count: SCENEKIT_CORE.len() + SCENEKIT_MATERIALS.len() + SCENEKIT_ANIMATION.len() + SCENEKIT_PHYSICS.len(),
        };

        let realitykit = GameDevTechnology {
            identifier: "gamedev:realitykit".to_string(),
            title: "RealityKit".to_string(),
            description: format!(
                "RealityKit AR/VR Engine - {} types for entities, components, and AR features",
                REALITYKIT_CORE.len() + REALITYKIT_COMPONENTS.len() + REALITYKIT_AR.len()
            ),
            url: REALITYKIT_URL.to_string(),
            item_count: REALITYKIT_CORE.len() + REALITYKIT_COMPONENTS.len() + REALITYKIT_AR.len(),
        };

        let gamekit = GameDevTechnology {
            identifier: "gamedev:gamekit".to_string(),
            title: "GameKit".to_string(),
            description: format!(
                "GameKit Multiplayer - {} types for Game Center, matchmaking, and social features",
                GAMEKIT_CORE.len() + GAMEKIT_MULTIPLAYER.len()
            ),
            url: GAMEKIT_URL.to_string(),
            item_count: GAMEKIT_CORE.len() + GAMEKIT_MULTIPLAYER.len(),
        };

        let gamecontroller = GameDevTechnology {
            identifier: "gamedev:gamecontroller".to_string(),
            title: "GameController".to_string(),
            description: format!(
                "GameController Input - {} types for gamepads, keyboards, and mice",
                GAMECONTROLLER_CORE.len()
            ),
            url: GAMECONTROLLER_URL.to_string(),
            item_count: GAMECONTROLLER_CORE.len(),
        };

        let patterns = GameDevTechnology {
            identifier: "gamedev:patterns".to_string(),
            title: "Game Patterns".to_string(),
            description: format!(
                "Game Development Patterns - {} patterns for ECS, state machines, AI, and pathfinding",
                GAMEDEV_PATTERNS.len()
            ),
            url: GAMEPLAYKIT_URL.to_string(),
            item_count: GAMEDEV_PATTERNS.len(),
        };

        let optimization = GameDevTechnology {
            identifier: "gamedev:optimization".to_string(),
            title: "Optimization".to_string(),
            description: format!(
                "Game Optimization - {} techniques for performance and memory",
                GAMEDEV_OPTIMIZATION.len()
            ),
            url: "https://developer.apple.com/documentation/xcode/improving-your-app-s-performance".to_string(),
            item_count: GAMEDEV_OPTIMIZATION.len(),
        };

        Ok(vec![spritekit, scenekit, realitykit, gamekit, gamecontroller, patterns, optimization])
    }

    /// Get a category of methods
    #[instrument(name = "gamedev_client.get_category", skip(self))]
    pub async fn get_category(&self, identifier: &str) -> Result<GameDevCategory> {
        let (methods, title, description): (Vec<&GameDevMethodIndex>, &str, &str) = match identifier {
            "gamedev:spritekit" | "spritekit" | "2d" | "sprite" => {
                let methods: Vec<&GameDevMethodIndex> = SPRITEKIT_CORE.iter()
                    .chain(SPRITEKIT_ACTIONS.iter())
                    .chain(SPRITEKIT_PHYSICS.iter())
                    .collect();
                (methods, "SpriteKit", "2D game engine with sprites, actions, and physics")
            }
            "spritekit:core" | "sknode" | "skscene" => (
                SPRITEKIT_CORE.iter().collect(),
                "SpriteKit Core",
                "Scene, nodes, sprites, labels, and shapes",
            ),
            "spritekit:actions" | "skaction" | "actions" => (
                SPRITEKIT_ACTIONS.iter().collect(),
                "SpriteKit Actions",
                "Movement, rotation, scaling, sequences, and animations",
            ),
            "spritekit:physics" | "physics2d" => (
                SPRITEKIT_PHYSICS.iter().collect(),
                "SpriteKit Physics",
                "2D physics world, bodies, contacts, and fields",
            ),
            "gamedev:scenekit" | "scenekit" | "3d" => {
                let methods: Vec<&GameDevMethodIndex> = SCENEKIT_CORE.iter()
                    .chain(SCENEKIT_MATERIALS.iter())
                    .chain(SCENEKIT_ANIMATION.iter())
                    .chain(SCENEKIT_PHYSICS.iter())
                    .collect();
                (methods, "SceneKit", "3D game engine with nodes, materials, and physics")
            }
            "scenekit:core" | "scnnode" | "scnscene" => (
                SCENEKIT_CORE.iter().collect(),
                "SceneKit Core",
                "Scene, nodes, geometry, cameras, and lights",
            ),
            "scenekit:materials" | "material" | "pbr" => (
                SCENEKIT_MATERIALS.iter().collect(),
                "SceneKit Materials",
                "PBR materials, textures, and custom shaders",
            ),
            "scenekit:animation" | "animation3d" => (
                SCENEKIT_ANIMATION.iter().collect(),
                "SceneKit Animation",
                "Actions, skeletal animation, morphing, and constraints",
            ),
            "scenekit:physics" | "physics3d" => (
                SCENEKIT_PHYSICS.iter().collect(),
                "SceneKit Physics",
                "3D physics world, bodies, shapes, and vehicles",
            ),
            "gamedev:realitykit" | "realitykit" | "ar" | "vr" | "visionos" => {
                let methods: Vec<&GameDevMethodIndex> = REALITYKIT_CORE.iter()
                    .chain(REALITYKIT_COMPONENTS.iter())
                    .chain(REALITYKIT_AR.iter())
                    .collect();
                (methods, "RealityKit", "AR/VR game engine for iOS and visionOS")
            }
            "realitykit:core" | "entity" => (
                REALITYKIT_CORE.iter().collect(),
                "RealityKit Core",
                "Entities, models, anchors, and scenes",
            ),
            "realitykit:components" | "component" => (
                REALITYKIT_COMPONENTS.iter().collect(),
                "RealityKit Components",
                "Transform, physics, collision, audio, and particles",
            ),
            "realitykit:ar" | "arkit" | "tracking" => (
                REALITYKIT_AR.iter().collect(),
                "RealityKit AR",
                "World tracking, anchoring, scene understanding, and hand tracking",
            ),
            "gamedev:gamekit" | "gamekit" | "multiplayer" | "gamecenter" => {
                let methods: Vec<&GameDevMethodIndex> = GAMEKIT_CORE.iter()
                    .chain(GAMEKIT_MULTIPLAYER.iter())
                    .collect();
                (methods, "GameKit", "Game Center integration and multiplayer")
            }
            "gamekit:core" | "achievements" | "leaderboards" => (
                GAMEKIT_CORE.iter().collect(),
                "GameKit Core",
                "Player authentication, achievements, and leaderboards",
            ),
            "gamekit:multiplayer" | "matchmaking" => (
                GAMEKIT_MULTIPLAYER.iter().collect(),
                "GameKit Multiplayer",
                "Real-time and turn-based multiplayer",
            ),
            "gamedev:gamecontroller" | "gamecontroller" | "input" | "controller" => (
                GAMECONTROLLER_CORE.iter().collect(),
                "GameController",
                "Gamepad, keyboard, and mouse input",
            ),
            "gamedev:patterns" | "patterns" | "gameplaykit" | "ecs" | "ai" => (
                GAMEDEV_PATTERNS.iter().collect(),
                "Game Patterns",
                "ECS, state machines, AI behaviors, and pathfinding",
            ),
            "gamedev:optimization" | "optimization" | "performance" => (
                GAMEDEV_OPTIMIZATION.iter().collect(),
                "Optimization",
                "Performance techniques for games",
            ),
            _ => anyhow::bail!("Unknown game dev category: {identifier}"),
        };

        let items = methods
            .iter()
            .map(|m| GameDevCategoryItem {
                name: m.name.to_string(),
                description: m.description.to_string(),
                kind: m.kind,
                framework: m.framework,
                url: self.get_method_url(m),
            })
            .collect();

        Ok(GameDevCategory {
            identifier: identifier.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            items,
        })
    }

    /// Get URL for a method
    fn get_method_url(&self, method: &GameDevMethodIndex) -> String {
        match method.framework {
            GameDevFramework::SpriteKit => format!("{}/{}", SPRITEKIT_URL, method.name.to_lowercase()),
            GameDevFramework::SceneKit => format!("{}/{}", SCENEKIT_URL, method.name.to_lowercase()),
            GameDevFramework::RealityKit => format!("{}/{}", REALITYKIT_URL, method.name.to_lowercase()),
            GameDevFramework::GameKit => format!("{}/{}", GAMEKIT_URL, method.name.to_lowercase()),
            GameDevFramework::GameController => format!("{}/{}", GAMECONTROLLER_URL, method.name.to_lowercase()),
            GameDevFramework::General => format!("{}/{}", GAMEPLAYKIT_URL, method.name.to_lowercase()),
        }
    }

    /// Get all methods as a flat list for searching
    fn all_methods() -> impl Iterator<Item = &'static GameDevMethodIndex> {
        SPRITEKIT_CORE.iter()
            .chain(SPRITEKIT_ACTIONS.iter())
            .chain(SPRITEKIT_PHYSICS.iter())
            .chain(SCENEKIT_CORE.iter())
            .chain(SCENEKIT_MATERIALS.iter())
            .chain(SCENEKIT_ANIMATION.iter())
            .chain(SCENEKIT_PHYSICS.iter())
            .chain(REALITYKIT_CORE.iter())
            .chain(REALITYKIT_COMPONENTS.iter())
            .chain(REALITYKIT_AR.iter())
            .chain(GAMEKIT_CORE.iter())
            .chain(GAMEKIT_MULTIPLAYER.iter())
            .chain(GAMECONTROLLER_CORE.iter())
            .chain(GAMEDEV_PATTERNS.iter())
            .chain(GAMEDEV_OPTIMIZATION.iter())
    }

    /// Build detailed method documentation
    fn build_method_doc(&self, index_entry: &GameDevMethodIndex) -> GameDevMethod {
        let examples = self.generate_examples(index_entry);
        let parameters = self.infer_parameters(index_entry);
        let platforms = self.get_platforms(index_entry);

        GameDevMethod {
            name: index_entry.name.to_string(),
            description: index_entry.description.to_string(),
            kind: index_entry.kind,
            framework: index_entry.framework,
            url: self.get_method_url(index_entry),
            parameters,
            returns: None,
            examples,
            platforms,
        }
    }

    /// Get platforms for a method
    fn get_platforms(&self, method: &GameDevMethodIndex) -> Vec<String> {
        match method.framework {
            GameDevFramework::SpriteKit | GameDevFramework::SceneKit => vec![
                "macOS 10.10+".to_string(),
                "iOS 8.0+".to_string(),
                "tvOS 9.0+".to_string(),
                "watchOS 3.0+".to_string(),
            ],
            GameDevFramework::RealityKit => vec![
                "macOS 10.15+".to_string(),
                "iOS 13.0+".to_string(),
                "visionOS 1.0+".to_string(),
            ],
            GameDevFramework::GameKit => vec![
                "macOS 10.8+".to_string(),
                "iOS 4.1+".to_string(),
                "tvOS 9.0+".to_string(),
            ],
            GameDevFramework::GameController => vec![
                "macOS 10.9+".to_string(),
                "iOS 7.0+".to_string(),
                "tvOS 9.0+".to_string(),
            ],
            GameDevFramework::General => vec![
                "macOS 10.11+".to_string(),
                "iOS 9.0+".to_string(),
                "tvOS 9.0+".to_string(),
            ],
        }
    }

    /// Generate example code for a method
    fn generate_examples(&self, method: &GameDevMethodIndex) -> Vec<GameDevExample> {
        let mut examples = Vec::new();

        match method.name {
            // SpriteKit
            "SKScene" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"class GameScene: SKScene {
    override func didMove(to view: SKView) {
        // Setup scene when presented
        backgroundColor = .black
        physicsWorld.gravity = CGVector(dx: 0, dy: -9.8)
        physicsWorld.contactDelegate = self

        // Create player sprite
        let player = SKSpriteNode(color: .blue, size: CGSize(width: 50, height: 50))
        player.position = CGPoint(x: frame.midX, y: frame.midY)
        player.physicsBody = SKPhysicsBody(rectangleOf: player.size)
        addChild(player)
    }

    override func update(_ currentTime: TimeInterval) {
        // Called before each frame is rendered
        // Game logic goes here
    }

    override func didSimulatePhysics() {
        // Called after physics simulation
        // Good for camera following, boundary checks
    }
}"#.to_string(),
                    description: Some("Basic SpriteKit scene setup".to_string()),
                });
            }
            "SKAction" | "SKAction.sequence" | "SKAction.group" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Move and fade animation sequence
let moveUp = SKAction.moveBy(x: 0, y: 100, duration: 0.5)
let fadeOut = SKAction.fadeOut(withDuration: 0.3)
let remove = SKAction.removeFromParent()
let sequence = SKAction.sequence([moveUp, fadeOut, remove])
sprite.run(sequence)

// Simultaneous actions
let rotate = SKAction.rotate(byAngle: .pi, duration: 0.5)
let scale = SKAction.scale(by: 2.0, duration: 0.5)
let group = SKAction.group([rotate, scale])
sprite.run(group)

// Repeating actions
let pulse = SKAction.sequence([
    SKAction.scale(to: 1.2, duration: 0.2),
    SKAction.scale(to: 1.0, duration: 0.2)
])
sprite.run(SKAction.repeatForever(pulse))

// Custom easing
let move = SKAction.move(to: target, duration: 0.5)
move.timingMode = .easeInEaseOut
sprite.run(move)"#.to_string(),
                    description: Some("SpriteKit actions for animation".to_string()),
                });
            }
            "SKPhysicsBody" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Physics body from shape
let body = SKPhysicsBody(rectangleOf: sprite.size)
body.isDynamic = true
body.mass = 1.0
body.friction = 0.5
body.restitution = 0.3  // Bounciness
body.linearDamping = 0.1
body.angularDamping = 0.1

// Category bit masks for collision detection
struct PhysicsCategory {
    static let player: UInt32 = 0x1 << 0   // 1
    static let enemy: UInt32 = 0x1 << 1    // 2
    static let bullet: UInt32 = 0x1 << 2   // 4
    static let wall: UInt32 = 0x1 << 3     // 8
}

body.categoryBitMask = PhysicsCategory.player
body.collisionBitMask = PhysicsCategory.wall | PhysicsCategory.enemy
body.contactTestBitMask = PhysicsCategory.enemy | PhysicsCategory.bullet

sprite.physicsBody = body

// Handle contacts
extension GameScene: SKPhysicsContactDelegate {
    func didBegin(_ contact: SKPhysicsContact) {
        let collision = contact.bodyA.categoryBitMask | contact.bodyB.categoryBitMask
        if collision == PhysicsCategory.player | PhysicsCategory.enemy {
            // Player hit enemy
            handlePlayerDamage()
        }
    }
}"#.to_string(),
                    description: Some("Physics body setup with collision detection".to_string()),
                });
            }

            // SceneKit
            "SCNScene" | "SCNView" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Setup SceneKit view
let sceneView = SCNView(frame: view.bounds)
sceneView.scene = SCNScene()
sceneView.allowsCameraControl = true  // Debug controls
sceneView.showsStatistics = true      // FPS counter
view.addSubview(sceneView)

// Add a camera
let cameraNode = SCNNode()
cameraNode.camera = SCNCamera()
cameraNode.position = SCNVector3(0, 5, 10)
cameraNode.look(at: SCNVector3.zero)
sceneView.scene?.rootNode.addChildNode(cameraNode)

// Add lighting
let lightNode = SCNNode()
lightNode.light = SCNLight()
lightNode.light?.type = .directional
lightNode.light?.intensity = 1000
lightNode.eulerAngles = SCNVector3(-Float.pi/4, 0, 0)
sceneView.scene?.rootNode.addChildNode(lightNode)

// Add ambient light
let ambientLight = SCNNode()
ambientLight.light = SCNLight()
ambientLight.light?.type = .ambient
ambientLight.light?.intensity = 300
sceneView.scene?.rootNode.addChildNode(ambientLight)"#.to_string(),
                    description: Some("Basic SceneKit scene setup".to_string()),
                });
            }
            "SCNMaterial" | "metalness" | "roughness" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// PBR material setup
let material = SCNMaterial()
material.lightingModel = .physicallyBased

// Textures
material.diffuse.contents = UIImage(named: "albedo")
material.normal.contents = UIImage(named: "normal")
material.roughness.contents = UIImage(named: "roughness")
material.metalness.contents = UIImage(named: "metalness")
material.ambientOcclusion.contents = UIImage(named: "ao")

// Or use values for simple materials
material.diffuse.contents = UIColor.red
material.metalness.contents = 0.8  // 0 = dielectric, 1 = metal
material.roughness.contents = 0.2  // 0 = mirror, 1 = diffuse

// Apply to geometry
let sphere = SCNSphere(radius: 1.0)
sphere.materials = [material]

// Environment for reflections
sceneView.scene?.lightingEnvironment.contents = UIImage(named: "environment.hdr")
sceneView.scene?.lightingEnvironment.intensity = 1.0"#.to_string(),
                    description: Some("PBR material configuration".to_string()),
                });
            }

            // RealityKit
            "Entity" | "ModelEntity" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Load a USDZ model
guard let entity = try? Entity.load(named: "robot") else { return }
entity.position = [0, 0, -2]
entity.scale = [0.5, 0.5, 0.5]

// Add physics
entity.components.set(PhysicsBodyComponent(
    massProperties: .default,
    material: .default,
    mode: .dynamic
))
entity.components.set(CollisionComponent(shapes: [.generateConvex(from: entity.model!.mesh)]))

// Add to anchor
let anchor = AnchorEntity(plane: .horizontal)
anchor.addChild(entity)
arView.scene.anchors.append(anchor)

// Create procedural geometry
let box = ModelEntity(
    mesh: .generateBox(size: 0.2),
    materials: [SimpleMaterial(color: .blue, isMetallic: true)]
)

// SwiftUI integration
struct ContentView: View {
    var body: some View {
        RealityView { content in
            let entity = try! Entity.load(named: "model")
            content.add(entity)
        }
    }
}"#.to_string(),
                    description: Some("RealityKit entity creation".to_string()),
                });
            }

            // GameKit
            "GKLocalPlayer" | "GKAchievement" | "GKLeaderboard" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Authenticate Game Center player
GKLocalPlayer.local.authenticateHandler = { viewController, error in
    if let vc = viewController {
        // Show login UI
        present(vc, animated: true)
    } else if GKLocalPlayer.local.isAuthenticated {
        // Player is authenticated
        print("Welcome \(GKLocalPlayer.local.displayName)")
    } else {
        // Handle error
        print("Game Center unavailable: \(error?.localizedDescription ?? "")")
    }
}

// Report achievement
let achievement = GKAchievement(identifier: "first_kill")
achievement.percentComplete = 100
achievement.showsCompletionBanner = true

GKAchievement.report([achievement]) { error in
    if let error = error {
        print("Failed to report achievement: \(error)")
    }
}

// Submit score to leaderboard
GKLeaderboard.submitScore(1000, context: 0, player: GKLocalPlayer.local, leaderboardIDs: ["high_scores"]) { error in
    if let error = error {
        print("Failed to submit score: \(error)")
    }
}"#.to_string(),
                    description: Some("Game Center authentication and reporting".to_string()),
                });
            }
            "GKMatch" | "GKMatchmaker" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Create match request
let request = GKMatchRequest()
request.minPlayers = 2
request.maxPlayers = 4
request.playerGroup = 0  // All players in same group

// Show matchmaker UI
let matchmakerVC = GKMatchmakerViewController(matchRequest: request)!
matchmakerVC.matchmakerDelegate = self
present(matchmakerVC, animated: true)

// Handle match
extension GameViewController: GKMatchmakerViewControllerDelegate {
    func matchmakerViewController(_ viewController: GKMatchmakerViewController, didFind match: GKMatch) {
        viewController.dismiss(animated: true)
        match.delegate = self
        startGame(with: match)
    }
}

// Send data to players
extension GameViewController: GKMatchDelegate {
    func sendGameState(_ state: GameState) {
        let data = try! JSONEncoder().encode(state)
        try? match.sendData(toAllPlayers: data, with: .reliable)
    }

    func match(_ match: GKMatch, didReceive data: Data, fromRemotePlayer player: GKPlayer) {
        let state = try! JSONDecoder().decode(GameState.self, from: data)
        handleRemoteState(state)
    }
}"#.to_string(),
                    description: Some("Real-time multiplayer with GameKit".to_string()),
                });
            }

            // GameController
            "GCController" | "GCExtendedGamepad" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Watch for controller connections
NotificationCenter.default.addObserver(
    forName: .GCControllerDidConnect,
    object: nil,
    queue: .main
) { notification in
    if let controller = notification.object as? GCController {
        setupController(controller)
    }
}

func setupController(_ controller: GCController) {
    guard let gamepad = controller.extendedGamepad else { return }

    // Button handlers
    gamepad.buttonA.pressedChangedHandler = { button, value, pressed in
        if pressed {
            jump()
        }
    }

    // Thumbstick for movement
    gamepad.leftThumbstick.valueChangedHandler = { stick, xValue, yValue in
        movePlayer(x: xValue, y: yValue)
    }

    // Triggers
    gamepad.rightTrigger.valueChangedHandler = { trigger, value, pressed in
        accelerate(amount: value)
    }
}

// Poll current state (alternative to callbacks)
func update() {
    guard let gamepad = GCController.current?.extendedGamepad else { return }

    let moveX = gamepad.leftThumbstick.xAxis.value
    let moveY = gamepad.leftThumbstick.yAxis.value
    movePlayer(x: moveX, y: moveY)

    if gamepad.buttonA.isPressed {
        jump()
    }
}"#.to_string(),
                    description: Some("Game controller input handling".to_string()),
                });
            }

            // Patterns
            "GKEntity" | "GKComponent" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// Entity Component System with GameplayKit

// Health component
class HealthComponent: GKComponent {
    var currentHealth: Float = 100
    var maxHealth: Float = 100

    func takeDamage(_ amount: Float) {
        currentHealth = max(0, currentHealth - amount)
        if currentHealth == 0 {
            entity?.component(ofType: RenderComponent.self)?.node.removeFromParent()
        }
    }
}

// Movement component
class MovementComponent: GKComponent {
    var speed: Float = 100

    override func update(deltaTime: TimeInterval) {
        guard let render = entity?.component(ofType: RenderComponent.self) else { return }
        // Update position based on input/AI
    }
}

// Render component bridging to SpriteKit
class RenderComponent: GKComponent {
    let node: SKNode

    init(node: SKNode) {
        self.node = node
        super.init()
    }
}

// Create entity
let player = GKEntity()
player.addComponent(HealthComponent())
player.addComponent(MovementComponent())
player.addComponent(RenderComponent(node: playerSprite))

// Update all movement components together
let movementSystem = GKComponentSystem(componentClass: MovementComponent.self)
movementSystem.addComponent(foundIn: player)

func update(_ deltaTime: TimeInterval) {
    movementSystem.update(deltaTime: deltaTime)
}"#.to_string(),
                    description: Some("Entity Component System pattern".to_string()),
                });
            }
            "GKStateMachine" | "GKState" => {
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: r#"// State machine for game flow
class MenuState: GKState {
    override func isValidNextState(_ stateClass: AnyClass) -> Bool {
        return stateClass == PlayingState.self
    }

    override func didEnter(from previousState: GKState?) {
        showMainMenu()
    }
}

class PlayingState: GKState {
    override func isValidNextState(_ stateClass: AnyClass) -> Bool {
        return stateClass == PausedState.self || stateClass == GameOverState.self
    }

    override func didEnter(from previousState: GKState?) {
        startGameplay()
    }

    override func update(deltaTime: TimeInterval) {
        updateGameLogic(deltaTime)
    }
}

class PausedState: GKState {
    override func isValidNextState(_ stateClass: AnyClass) -> Bool {
        return stateClass == PlayingState.self || stateClass == MenuState.self
    }
}

class GameOverState: GKState {
    override func isValidNextState(_ stateClass: AnyClass) -> Bool {
        return stateClass == MenuState.self
    }
}

// Create state machine
let stateMachine = GKStateMachine(states: [
    MenuState(),
    PlayingState(),
    PausedState(),
    GameOverState()
])

stateMachine.enter(MenuState.self)

// Transition
func startGame() {
    stateMachine.enter(PlayingState.self)
}

func pauseGame() {
    stateMachine.enter(PausedState.self)
}"#.to_string(),
                    description: Some("State machine for game flow".to_string()),
                });
            }

            _ => {
                // Generic example
                let framework_url = match method.framework {
                    GameDevFramework::SpriteKit => SPRITEKIT_URL,
                    GameDevFramework::SceneKit => SCENEKIT_URL,
                    GameDevFramework::RealityKit => REALITYKIT_URL,
                    GameDevFramework::GameKit => GAMEKIT_URL,
                    GameDevFramework::GameController => GAMECONTROLLER_URL,
                    GameDevFramework::General => GAMEPLAYKIT_URL,
                };
                examples.push(GameDevExample {
                    language: "swift".to_string(),
                    code: format!(
                        "// See Apple Developer Documentation for {}\n// {}/{}",
                        method.name, framework_url, method.name.to_lowercase()
                    ),
                    description: Some(format!("{} documentation", method.framework)),
                });
            }
        }

        examples
    }

    /// Infer parameters for a method
    fn infer_parameters(&self, _method: &GameDevMethodIndex) -> Vec<GameDevParameter> {
        Vec::new()
    }

    /// Get a specific method by name
    #[instrument(name = "gamedev_client.get_method", skip(self))]
    pub async fn get_method(&self, name: &str) -> Result<GameDevMethod> {
        let index_entry = Self::all_methods()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("Game dev method not found: {name}"))?;

        Ok(self.build_method_doc(index_entry))
    }

    /// Search for methods matching a query
    #[instrument(name = "gamedev_client.search", skip(self))]
    pub async fn search(&self, query: &str) -> Result<Vec<GameDevMethod>> {
        let query_lower = query.to_lowercase();

        let keywords: Vec<&str> = query_lower
            .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .collect();

        let mut scored_results: Vec<(i32, &GameDevMethodIndex)> = Vec::new();

        for method in Self::all_methods() {
            let name_lower = method.name.to_lowercase();
            let desc_lower = method.description.to_lowercase();
            let category_lower = method.category.to_lowercase();
            let framework_lower = method.framework.to_string().to_lowercase();

            let mut score = 0i32;

            for keyword in &keywords {
                if name_lower == *keyword {
                    score += 50;
                } else if name_lower.contains(keyword) {
                    score += 20;
                }
                if category_lower.contains(keyword) || framework_lower.contains(keyword) {
                    score += 10;
                }
                if desc_lower.contains(keyword) {
                    score += 5;
                }
            }

            // Framework-specific boosts
            if query_lower.contains("spritekit") || query_lower.contains("2d") || query_lower.contains("sprite") {
                if method.framework == GameDevFramework::SpriteKit {
                    score += 15;
                }
            }
            if query_lower.contains("scenekit") || query_lower.contains("3d") {
                if method.framework == GameDevFramework::SceneKit {
                    score += 15;
                }
            }
            if query_lower.contains("reality") || query_lower.contains("ar") || query_lower.contains("vr") || query_lower.contains("visionos") {
                if method.framework == GameDevFramework::RealityKit {
                    score += 15;
                }
            }
            if query_lower.contains("multiplayer") || query_lower.contains("gamecenter") {
                if method.framework == GameDevFramework::GameKit {
                    score += 15;
                }
            }
            if query_lower.contains("controller") || query_lower.contains("gamepad") || query_lower.contains("input") {
                if method.framework == GameDevFramework::GameController {
                    score += 15;
                }
            }

            if score > 0 {
                scored_results.push((score, method));
            }
        }

        scored_results.sort_by(|a, b| b.0.cmp(&a.0));

        let results: Vec<GameDevMethod> = scored_results
            .into_iter()
            .take(20)
            .map(|(_, m)| self.build_method_doc(m))
            .collect();

        Ok(results)
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = GameDevClient::new();
    }

    #[test]
    fn test_all_methods_count() {
        let count = GameDevClient::all_methods().count();
        assert!(count > 80, "Expected at least 80 methods, got {}", count);
    }
}
