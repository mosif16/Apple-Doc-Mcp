use serde::{Deserialize, Serialize};

// ============================================================================
// GAME DEVELOPMENT DOCUMENTATION PROVIDER
// ============================================================================
//
// Apple Game Development frameworks for iOS, macOS, tvOS, and visionOS.
// Covers 2D games (SpriteKit), 3D games (SceneKit), AR/VR (RealityKit),
// multiplayer (GameKit), and game controllers.
//
// Frameworks:
// - SpriteKit: 2D game engine with physics, particles, actions
// - SceneKit: 3D game engine with physics, materials, animations
// - RealityKit: AR/VR framework with realistic rendering
// - GameKit: Multiplayer, leaderboards, achievements
// - GameController: MFi controllers, keyboard, mouse input
//
// Documentation Sources:
// - https://developer.apple.com/documentation/spritekit
// - https://developer.apple.com/documentation/scenekit
// - https://developer.apple.com/documentation/realitykit
// - https://developer.apple.com/documentation/gamekit
// - https://developer.apple.com/documentation/gamecontroller
//
// ============================================================================

/// Game development technology representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevTechnology {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub item_count: usize,
}

/// Category of game development documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevCategory {
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub items: Vec<GameDevCategoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevCategoryItem {
    pub name: String,
    pub description: String,
    pub kind: GameDevItemKind,
    pub url: String,
    pub framework: GameDevFramework,
}

/// Framework within game development
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameDevFramework {
    SpriteKit,
    SceneKit,
    RealityKit,
    GameKit,
    GameController,
    General,
}

impl std::fmt::Display for GameDevFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl GameDevFramework {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SpriteKit => "SpriteKit",
            Self::SceneKit => "SceneKit",
            Self::RealityKit => "RealityKit",
            Self::GameKit => "GameKit",
            Self::GameController => "GameController",
            Self::General => "General",
        }
    }
}

/// Kind of game development documentation item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameDevItemKind {
    /// Core type/class
    CoreType,
    /// Node type (SKNode, SCNNode, etc.)
    Node,
    /// Action or animation
    Action,
    /// Physics component
    Physics,
    /// Material or shader
    Material,
    /// Audio component
    Audio,
    /// Input handling
    Input,
    /// Multiplayer/networking
    Multiplayer,
    /// AR/VR component
    AR,
    /// Best practice or pattern
    Pattern,
    /// Performance optimization
    Optimization,
}

impl std::fmt::Display for GameDevItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoreType => write!(f, "Core Type"),
            Self::Node => write!(f, "Node"),
            Self::Action => write!(f, "Action"),
            Self::Physics => write!(f, "Physics"),
            Self::Material => write!(f, "Material"),
            Self::Audio => write!(f, "Audio"),
            Self::Input => write!(f, "Input"),
            Self::Multiplayer => write!(f, "Multiplayer"),
            Self::AR => write!(f, "AR/VR"),
            Self::Pattern => write!(f, "Pattern"),
            Self::Optimization => write!(f, "Optimization"),
        }
    }
}

/// Detailed game development API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevMethod {
    pub name: String,
    pub description: String,
    pub kind: GameDevItemKind,
    pub framework: GameDevFramework,
    pub url: String,
    pub parameters: Vec<GameDevParameter>,
    pub returns: Option<GameDevReturnType>,
    pub examples: Vec<GameDevExample>,
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevReturnType {
    pub type_name: String,
    pub description: String,
    pub fields: Vec<GameDevReturnField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevReturnField {
    pub name: String,
    pub field_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDevExample {
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Static method index entry
#[derive(Debug, Clone)]
pub struct GameDevMethodIndex {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: GameDevItemKind,
    pub framework: GameDevFramework,
    pub category: &'static str,
}

// ============================================================================
// SPRITEKIT - 2D GAME ENGINE
// ============================================================================

pub const SPRITEKIT_CORE: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SKView", description: "A view that renders SpriteKit content. Set the scene property to display a scene. Configure showsFPS, showsNodeCount for debugging.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKScene", description: "The root node of a SpriteKit scene graph. Override didMove(to:), update(:), and didSimulatePhysics() for game logic.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKNode", description: "Base class for all SpriteKit scene graph elements. Nodes have position, zPosition, rotation, scale, and can run actions.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKSpriteNode", description: "A node that displays an image or solid color. Use texture property for images, size and color for rectangles.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKLabelNode", description: "A node that displays text. Configure fontName, fontSize, fontColor, and alignment.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKShapeNode", description: "A node that renders a Core Graphics path. Create circles, rectangles, custom shapes with stroke and fill.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKCameraNode", description: "A node that defines the visible area of the scene. Use constraints to follow the player.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKLightNode", description: "A node that provides lighting for other nodes. Ambient and specular lighting with falloff.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKEffectNode", description: "A node that applies Core Image filters to its children. Use for blur, glow, color adjustments.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKCropNode", description: "A node that masks its children using another node. Create irregular shaped clipping regions.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKReferenceNode", description: "A node that loads its contents from a separate .sks file. Reusable prefabs.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKTileMapNode", description: "A node that displays a tile map. Efficient rendering of 2D tile-based levels.", kind: GameDevItemKind::Node, framework: GameDevFramework::SpriteKit, category: "spritekit" },
];

pub const SPRITEKIT_ACTIONS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SKAction", description: "An action that changes a node over time. Actions can be chained, grouped, or repeated.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.move", description: "Moves a node to a position or by a delta over duration. Use timingMode for easing.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.rotate", description: "Rotates a node to an angle or by a delta over duration.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.scale", description: "Scales a node to a value or by a factor over duration.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.fadeIn", description: "Fades a node to full opacity over duration.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.fadeOut", description: "Fades a node to zero opacity over duration.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.sequence", description: "Runs an array of actions one after another.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.group", description: "Runs an array of actions simultaneously.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.repeatForever", description: "Repeats an action indefinitely until removed.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.wait", description: "Creates a delay. Use in sequences for timing.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.run", description: "Runs a custom closure. Execute game logic within action sequences.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.animate", description: "Animates between textures. Create sprite animations from a texture atlas.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.playSoundFileNamed", description: "Plays a sound effect. Non-blocking, fire-and-forget audio.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKAction.follow", description: "Moves a node along a CGPath over duration.", kind: GameDevItemKind::Action, framework: GameDevFramework::SpriteKit, category: "spritekit" },
];

pub const SPRITEKIT_PHYSICS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SKPhysicsWorld", description: "The physics simulation for a scene. Configure gravity, speed, and set contactDelegate for collision callbacks.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKPhysicsBody", description: "Defines the physical properties of a node. Create from shapes, textures, or paths. Set mass, friction, restitution.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKPhysicsContact", description: "Information about a collision between two bodies. Contains contact point, normal, and collision impulse.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKPhysicsJoint", description: "Base class for joints connecting physics bodies. Pin, spring, sliding, and fixed joints available.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "SKFieldNode", description: "Applies forces to physics bodies in its area. Gravity, vortex, noise, spring, and custom fields.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "categoryBitMask", description: "Defines which category a physics body belongs to. Used with collisionBitMask and contactTestBitMask.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "collisionBitMask", description: "Defines which categories this body collides with. Bodies bounce off matching categories.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
    GameDevMethodIndex { name: "contactTestBitMask", description: "Defines which categories trigger contact callbacks. Use for game logic without physical collision.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SpriteKit, category: "spritekit" },
];

// ============================================================================
// SCENEKIT - 3D GAME ENGINE
// ============================================================================

pub const SCENEKIT_CORE: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SCNView", description: "A view that renders SceneKit content. Set scene property, configure allowsCameraControl for debugging.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNScene", description: "The container for SceneKit content. Has a rootNode, physicsWorld, background, and lightingEnvironment.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNNode", description: "A structural element of a scene graph. Has position, rotation, scale, and can contain geometry, cameras, lights.", kind: GameDevItemKind::Node, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNGeometry", description: "A 3D shape with materials. Use built-in shapes (box, sphere, cylinder) or create custom geometry.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNBox", description: "A rectangular box geometry. Specify width, height, length, and chamfer radius.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNSphere", description: "A sphere geometry. Specify radius and segment counts.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNCapsule", description: "A capsule geometry (cylinder with hemispherical caps). Good for character colliders.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNCylinder", description: "A cylinder geometry. Specify radius and height.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPlane", description: "A rectangular plane geometry. Specify width and height. Single-sided by default.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNCamera", description: "A camera that defines the point of view. Configure fieldOfView, orthographic projection, DOF, motion blur.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNLight", description: "A light source. Types: ambient, directional, omni (point), spot. Shadows, intensity, color.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNReferenceNode", description: "A node loaded from an external file. Loads .scn, .dae, .usdz files as prefabs.", kind: GameDevItemKind::Node, framework: GameDevFramework::SceneKit, category: "scenekit" },
];

pub const SCENEKIT_MATERIALS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SCNMaterial", description: "Surface appearance for geometry. Configure diffuse, specular, emission, normal maps and PBR properties.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "diffuse", description: "The base color/texture of a material. Set contents to UIColor, UIImage, or SKTexture.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "specular", description: "The specular highlight color/texture. Affects shininess of the surface.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "normal", description: "Normal map for surface detail. Adds apparent geometry without adding polygons.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "metalness", description: "PBR metalness value (0-1). Metal surfaces reflect environment, non-metals show diffuse.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "roughness", description: "PBR roughness value (0-1). Smooth surfaces have sharp reflections, rough surfaces scatter light.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "lightingModel", description: "Rendering model: .physicallyBased (PBR), .blinn, .phong, .lambert, .constant (unlit).", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNProgram", description: "Custom Metal shaders for SceneKit geometry. Full control over vertex and fragment processing.", kind: GameDevItemKind::Material, framework: GameDevFramework::SceneKit, category: "scenekit" },
];

pub const SCENEKIT_ANIMATION: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SCNAction", description: "Actions for SceneKit nodes. Similar to SKAction: move, rotate, scale, fade, sequences, groups.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNTransaction", description: "Implicit animation for property changes. Wrap changes in begin/commit for automatic animation.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNAnimationPlayer", description: "Controls playback of animations. Play, pause, blend between animations.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNAnimation", description: "A Core Animation-compatible animation. Load from .dae or .scn files, or create programmatically.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNConstraint", description: "Automatic node positioning. LookAt, distance, IK constraints for character animation.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNLookAtConstraint", description: "Orients a node to always face another node. Useful for cameras and NPC heads.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNIKConstraint", description: "Inverse kinematics constraint. Chain of joints reaches toward a target position.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNSkinner", description: "Skeletal animation support. Binds geometry to a skeleton for character animation.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNMorpher", description: "Blend shape/morph target animation. Smooth transitions between geometry shapes for facial animation.", kind: GameDevItemKind::Action, framework: GameDevFramework::SceneKit, category: "scenekit" },
];

pub const SCENEKIT_PHYSICS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "SCNPhysicsWorld", description: "The physics simulation for a scene. Configure gravity, speed, and set contactDelegate.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPhysicsBody", description: "A physics body attached to a node. Types: .static, .dynamic, .kinematic.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPhysicsShape", description: "The collision shape for a physics body. Box, sphere, capsule, or convex hull from geometry.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPhysicsContact", description: "Information about a collision. Contact point, normal, penetration depth.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPhysicsField", description: "Force fields affecting physics bodies. Gravity, vortex, noise, turbulence, magnetic.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
    GameDevMethodIndex { name: "SCNPhysicsVehicle", description: "A vehicle behavior with wheels. Configurable suspension, steering, and engine force.", kind: GameDevItemKind::Physics, framework: GameDevFramework::SceneKit, category: "scenekit" },
];

// ============================================================================
// REALITYKIT - AR/VR GAME ENGINE
// ============================================================================

pub const REALITYKIT_CORE: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "ARView", description: "A view that renders RealityKit content with AR. Configure camera mode, environment texturing, and debug options.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "RealityView", description: "SwiftUI view for RealityKit content. Use make: and update: closures to manage entities.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "Entity", description: "The fundamental building block of RealityKit. Entities have components that define behavior and appearance.", kind: GameDevItemKind::Node, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ModelEntity", description: "An entity that displays 3D content. Load from .usdz or create with mesh and materials.", kind: GameDevItemKind::Node, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "AnchorEntity", description: "An entity anchored to the real world. Anchor to planes, images, faces, or specific positions.", kind: GameDevItemKind::Node, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "Experience.loadBox", description: "Load a Reality Composer scene. Returns the root entity with animations and behaviors.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "Scene", description: "The container for all RealityKit content. Add anchor entities to the scene.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
];

pub const REALITYKIT_COMPONENTS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "Component", description: "A unit of functionality attached to an entity. Transform, mesh, physics, collision, etc.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "Transform", description: "Position, rotation, and scale of an entity. Matrix-based for complex transformations.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ModelComponent", description: "Renders a 3D mesh with materials. Load from assets or create procedurally.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "PhysicsBodyComponent", description: "Physics simulation properties. Mass, mode (static, dynamic, kinematic), and motion.", kind: GameDevItemKind::Physics, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "CollisionComponent", description: "Collision detection shape. Box, sphere, capsule, or convex mesh.", kind: GameDevItemKind::Physics, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "CharacterControllerComponent", description: "Character movement with ground detection and stairs. Similar to Unity's CharacterController.", kind: GameDevItemKind::Physics, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "AudioFileResource", description: "Audio loaded from a file. Use with SpatialAudioComponent for 3D sound.", kind: GameDevItemKind::Audio, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "SpatialAudioComponent", description: "3D positional audio. Sound attenuates with distance and pans with position.", kind: GameDevItemKind::Audio, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "VideoPlayerComponent", description: "Video playback on an entity. Texture mapping for video on 3D objects.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ParticleEmitterComponent", description: "Particle system for effects. Fire, smoke, sparkles with customizable parameters.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::RealityKit, category: "realitykit" },
];

pub const REALITYKIT_AR: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "ARSession", description: "Manages the AR experience. Configure tracking, plane detection, and environment understanding.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ARWorldTrackingConfiguration", description: "6DOF world tracking configuration. Plane detection, ray casting, scene reconstruction.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "AnchoringComponent.Target", description: "Anchor target type. .plane, .image, .face, .body, .object, or .world.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ARRaycastQuery", description: "Cast a ray into the AR scene. Find real-world surfaces for object placement.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "SceneUnderstanding", description: "Mesh reconstruction of the environment. Enables occlusion, physics with real objects.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "ObjectCapture", description: "Create 3D models from photos. Photogrammetry pipeline for asset creation.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
    GameDevMethodIndex { name: "HandTrackingProvider", description: "Track user's hands in visionOS. Access joint positions, gestures.", kind: GameDevItemKind::AR, framework: GameDevFramework::RealityKit, category: "realitykit" },
];

// ============================================================================
// GAMEKIT - MULTIPLAYER & SOCIAL
// ============================================================================

pub const GAMEKIT_CORE: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "GKLocalPlayer", description: "The authenticated Game Center player. Check isAuthenticated, access friends, achievements.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKPlayer", description: "Information about a Game Center player. Display name, avatar, game-specific data.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKAchievement", description: "An achievement instance. Set percentComplete and report using report() method.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKAchievementDescription", description: "Metadata about an achievement. Title, description, and image.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKLeaderboard", description: "A leaderboard for score comparison. Load entries, submit scores, access player ranking.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKScore", description: "A score entry for a leaderboard. Value, context, and formatted display.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKAccessPoint", description: "Game Center access point UI. Shows player status and provides access to Game Center.", kind: GameDevItemKind::CoreType, framework: GameDevFramework::GameKit, category: "gamekit" },
];

pub const GAMEKIT_MULTIPLAYER: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "GKMatchmaker", description: "Creates multiplayer matches. Find players by skill, invite friends, or auto-match.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKMatch", description: "An active multiplayer match. Send data to players, handle disconnections.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKMatchRequest", description: "Configuration for matchmaking. Min/max players, player group, player attributes.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKMatchmakerViewController", description: "System UI for matchmaking. Shows connected players and match status.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKTurnBasedMatch", description: "A turn-based multiplayer match. Async gameplay like chess or word games.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKTurnBasedMatchmakerViewController", description: "UI for turn-based matchmaking. Shows active matches and invitations.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "send", description: "Send data to match players. Use .reliable for important data, .unreliable for frequent updates.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
    GameDevMethodIndex { name: "GKVoiceChat", description: "Voice communication in matches. Create channels, mute players, control volume.", kind: GameDevItemKind::Multiplayer, framework: GameDevFramework::GameKit, category: "gamekit" },
];

// ============================================================================
// GAME CONTROLLER - INPUT HANDLING
// ============================================================================

pub const GAMECONTROLLER_CORE: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "GCController", description: "A connected game controller. Access via GCController.controllers() or observe notifications.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCExtendedGamepad", description: "Extended gamepad profile. Two thumbsticks, dpad, four face buttons, shoulders, triggers.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCMicroGamepad", description: "Micro gamepad profile. Dpad and two buttons. Used by Siri Remote.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCKeyboard", description: "Keyboard input for games. Access individual keys and modifiers.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCMouse", description: "Mouse input for games. Delta movement, buttons, and scroll wheel.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCVirtualController", description: "On-screen virtual controller. Create custom button layouts.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "valueChangedHandler", description: "Callback for input changes. Assign to button/stick elements for event handling.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCControllerButtonInput", description: "A button on a controller. Read value (0-1), isPressed state.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCControllerAxisInput", description: "An axis on a controller (trigger, stick axis). Read value (-1 to 1).", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
    GameDevMethodIndex { name: "GCControllerDirectionPad", description: "A directional input (dpad, thumbstick). Read xAxis, yAxis, up/down/left/right.", kind: GameDevItemKind::Input, framework: GameDevFramework::GameController, category: "gamecontroller" },
];

// ============================================================================
// GAME DEVELOPMENT PATTERNS & OPTIMIZATION
// ============================================================================

pub const GAMEDEV_PATTERNS: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "game_loop", description: "The update-render cycle. SpriteKit: update(:), didSimulatePhysics(). SceneKit: renderer(:updateAtTime:).", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "entity_component_system", description: "ECS architecture. RealityKit uses components. For SpriteKit/SceneKit, use GKEntity and GKComponent.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKEntity", description: "GameplayKit entity. Add GKComponent instances for modular game object behavior.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKComponent", description: "GameplayKit component. Subclass to create reusable behaviors like health, movement, AI.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKComponentSystem", description: "Updates all components of a type together. Efficient batch processing.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKStateMachine", description: "Finite state machine. Manage game states (menu, playing, paused) or entity states (idle, attacking).", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKState", description: "A state in a state machine. Override isValidNextState, didEnter, willExit.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKRandom", description: "GameplayKit random sources. Use GKRandomDistribution for consistent, reproducible randomness.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKNoise", description: "Procedural noise generation. Perlin, Voronoi, billow for terrain, textures.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKAgent", description: "Autonomous movement. Seek, flee, wander, follow path, flocking behaviors.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKGoal", description: "Steering behavior for agents. Combine goals for complex AI movement.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKPath", description: "A path for pathfinding and agent movement. Create from points or navigation mesh.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKObstacle", description: "Obstacles for pathfinding. Circle, polygon from nodes or physics bodies.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKGraph", description: "Navigation graph for pathfinding. Grid, obstacle, or mesh graphs.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKMinmaxStrategist", description: "AI for turn-based games. Minimax with alpha-beta pruning for game tree search.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
    GameDevMethodIndex { name: "GKMonteCarloStrategist", description: "AI using Monte Carlo tree search. Good for games with large branching factors.", kind: GameDevItemKind::Pattern, framework: GameDevFramework::General, category: "patterns" },
];

pub const GAMEDEV_OPTIMIZATION: &[GameDevMethodIndex] = &[
    GameDevMethodIndex { name: "texture_atlas", description: "Combine sprites into a single texture. Reduces draw calls, improves batching.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::General, category: "optimization" },
    GameDevMethodIndex { name: "SKTextureAtlas", description: "SpriteKit texture atlas. Load from .atlas folder or create at runtime.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SpriteKit, category: "optimization" },
    GameDevMethodIndex { name: "shouldEnableEffects", description: "Enable/disable node effects. Disable effect nodes when not visible.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SpriteKit, category: "optimization" },
    GameDevMethodIndex { name: "isPaused", description: "Pause a node and its children. Stops actions and physics simulation.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SpriteKit, category: "optimization" },
    GameDevMethodIndex { name: "LOD", description: "Level of detail. Use simpler geometry for distant objects. SceneKit has SCNLevelOfDetail.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SceneKit, category: "optimization" },
    GameDevMethodIndex { name: "SCNLevelOfDetail", description: "Automatic geometry switching based on distance. Improves performance for complex scenes.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SceneKit, category: "optimization" },
    GameDevMethodIndex { name: "frustum_culling", description: "Don't render objects outside the camera view. Automatic in SceneKit/RealityKit.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::General, category: "optimization" },
    GameDevMethodIndex { name: "occlusion_culling", description: "Don't render objects behind other objects. Enable for complex scenes.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::General, category: "optimization" },
    GameDevMethodIndex { name: "instancing", description: "Draw many copies of the same geometry efficiently. Use SCNNode.clone() or instanced rendering.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::SceneKit, category: "optimization" },
    GameDevMethodIndex { name: "frame_rate", description: "Target 60fps (16.6ms) or 120fps (8.3ms) on ProMotion. Use preferredFramesPerSecond.", kind: GameDevItemKind::Optimization, framework: GameDevFramework::General, category: "optimization" },
];
