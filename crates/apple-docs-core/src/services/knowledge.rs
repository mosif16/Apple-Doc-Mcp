use once_cell::sync::Lazy;
use std::collections::HashMap;

pub struct IntegrationLink {
    pub framework: &'static str,
    pub title: &'static str,
    pub path: &'static str,
    pub note: &'static str,
}

#[derive(Clone)]
pub struct RelatedItem {
    pub title: &'static str,
    pub path: &'static str,
    pub note: &'static str,
}

pub struct KnowledgeEntry {
    pub quick_tip: Option<&'static str>,
    pub related: &'static [RelatedItem],
    pub integration: &'static [IntegrationLink],
    pub snippet: Option<Snippet>,
}

#[derive(Clone, Copy)]
pub struct Snippet {
    pub language: &'static str,
    pub code: &'static str,
    pub caption: Option<&'static str>,
}

pub struct RecipeDefinition {
    pub id: &'static str,
    pub technology: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub keywords: &'static [&'static str],
    pub steps: &'static [&'static str],
    pub references: &'static [RelatedItem],
}

impl RecipeDefinition {
    fn matches(&self, query: &str, technology: &str) -> bool {
        if !self.technology.eq_ignore_ascii_case(technology.trim()) {
            return false;
        }

        let normalized = query.to_lowercase();
        self.keywords
            .iter()
            .any(|keyword| normalized.contains(keyword.to_lowercase().as_str()))
    }
}

static KNOWLEDGE: Lazy<HashMap<&'static str, KnowledgeEntry>> = Lazy::new(|| {
    use IntegrationLink as Link;
    use KnowledgeEntry as Entry;
    use RelatedItem as Related;

    const SWIFTUI_SEARCHABLE_INTEGRATIONS: [Link; 2] = [
        Link {
            framework: "UIKit",
            title: "UISearchController",
            path: "/documentation/uikit/uisearchcontroller",
            note: "Embed when hosting SwiftUI search inside UIKit navigation stacks.",
        },
        Link {
            framework: "AppKit",
            title: "NSSearchToolbarItem",
            path: "/documentation/appkit/nssearchtoolbaritem",
            note: "Provide macOS toolbar search alongside SwiftUI content.",
        },
    ];
    const SWIFTUI_SEARCHABLE_RELATED: [Related; 2] = [
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Offer auto-complete suggestions as people type.",
        },
        Related {
            title: "searchScopes(_:scopes:)",
            path: "/documentation/swiftui/view/searchscopes(_:scopes:)",
            note: "Partition search results into categories.",
        },
    ];

    const SWIFTUI_TEXTFIELD_INTEGRATIONS: [Link; 2] = [
        Link {
            framework: "UIKit",
            title: "UITextField",
            path: "/documentation/UIKit/UITextField",
            note: "Leverage UIKit delegates when you need granular editing control.",
        },
        Link {
            framework: "AppKit",
            title: "NSTextField",
            path: "/documentation/AppKit/NSTextField",
            note: "Use for macOS-specific behaviors like formatter delegates.",
        },
    ];
    const SWIFTUI_TEXTFIELD_RELATED: [Related; 3] = [
        Related {
            title: "TextFieldStyle",
            path: "/documentation/swiftui/textfieldstyle",
            note: "Select styling presets that align with platform idioms.",
        },
        Related {
            title: "focused(_:equals:)",
            path: "/documentation/swiftui/view/focused(_:equals:)",
            note: "Manage focus programmatically for text inputs.",
        },
        Related {
            title: "TextInputAutocapitalization",
            path: "/documentation/swiftui/textinputautocapitalization",
            note: "Tune keyboard behavior for the field’s content.",
        },
    ];

    const SWIFTUI_TEXTFIELD_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "@State private var username = \"\"\n\nTextField(\"Username\", text: $username)\n    .textInputAutocapitalization(.never)\n    .textFieldStyle(.roundedBorder)",
        caption: Some("Bind text to state and customize keyboard behavior."),
    };

    const SWIFTUI_LIST_INTEGRATIONS: [Link; 1] = [Link {
        framework: "UIKit",
        title: "UITableView",
        path: "/documentation/uikit/uitableview",
        note: "Bridge to UIKit list controllers during incremental migration.",
    }];
    const SWIFTUI_LIST_RELATED: [Related; 3] = [
        Related {
            title: "refreshable(action:)",
            path: "/documentation/swiftui/view/refreshable(action:)",
            note: "Add pull-to-refresh to long lists.",
        },
        Related {
            title: "swipeActions(edge:allowsFullSwipe:content:)",
            path: "/documentation/swiftui/view/swipeactions(edge:allowsfullswipe:content:)",
            note: "Expose trailing actions that mirror UITableView behaviors.",
        },
        Related {
            title: "listRowSeparator(_:edges:)",
            path: "/documentation/swiftui/view/listrowseparator(_:edges:)",
            note: "Control separators for grouped or inset list styles.",
        },
    ];

    const SWIFTUI_LIST_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "List(filteredItems) { item in\n    Label(item.title, systemImage: item.icon)\n}\n.listStyle(.insetGrouped)",
        caption: Some("Filter data and apply a list style that matches the platform."),
    };

    const SWIFTUI_ACCESSIBILITY_RELATED: [Related; 2] = [
        Related {
            title: "accessibilityValue(_:)",
            path: "/documentation/swiftui/view/accessibilityvalue(_:)",
            note: "Describe dynamic values such as progress or selection.",
        },
        Related {
            title: "accessibilityHint(_:)",
            path: "/documentation/swiftui/view/accessibilityhint(_:)",
            note: "Explain the result of activating the element.",
        },
    ];

    const SWIFTUI_ACCESSIBILITY_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "Image(systemName: \"speaker.wave.2.fill\")\n    .accessibilityLabel(\"Playback volume\")\n    .accessibilityValue(\"70 percent\")\n    .accessibilityHint(\"Adjust with the volume buttons\")",
        caption: Some("Pair labels, values, and hints for richer VoiceOver output."),
    };

    const SWIFTUI_TEXT_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "Text(\"Welcome\")\n    .font(.title.bold())\n    .foregroundStyle(.primary)",
        caption: Some("Render static copy with typography that adapts to Dynamic Type."),
    };

    const SWIFTUI_SEARCH_TOPIC_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "@State private var searchText = \"\"\n\nNavigationStack {\n    List(results) { result in\n        Text(result.title)\n    }\n    .searchable(text: $searchText)\n    .searchSuggestions {\n        ForEach(suggestions) { suggestion in\n            Text(suggestion).searchCompletion(suggestion)\n        }\n    }\n}",
        caption: Some("Combine search field and suggestions inside navigation."),
    };

    const UIKIT_UITEXTFIELD_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "let textField = UITextField(frame: .zero)\ntextField.placeholder = \"Email address\"\ntextField.keyboardType = .emailAddress\ntextField.delegate = self",
        caption: Some("Configure delegates and text input traits for UIKit forms."),
    };

    const SWIFTUI_SEARCH_TOPIC_RELATED: [Related; 3] = [
        Related {
            title: "searchable(text:placement:prompt:)",
            path: "/documentation/swiftui/view/searchable(text:placement:prompt:)",
            note: "Enable the search field.",
        },
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Offer query completions.",
        },
        Related {
            title: "List",
            path: "/documentation/swiftui/list",
            note: "Display search results in scrollable content.",
        },
    ];

    const SWIFTUI_SEARCH_TOPIC_INTEGRATIONS: [Link; 1] = [Link {
        framework: "UIKit",
        title: "UISearchController",
        path: "/documentation/uikit/uisearchcontroller",
        note: "Embed SwiftUI search inside UIKit navigation stacks when iterating gradually.",
    }];

    const UIKIT_UITEXTFIELD_INTEGRATIONS: [Link; 1] = [Link {
        framework: "SwiftUI",
        title: "TextField",
        path: "/documentation/SwiftUI/TextField",
        note: "Embed SwiftUI text inputs with UIHostingController.",
    }];
    const UIKIT_UITEXTFIELD_RELATED: [Related; 2] = [
        Related {
            title: "UITextFieldDelegate",
            path: "/documentation/uikit/uitextfielddelegate",
            note: "Respond to editing events and validate input.",
        },
        Related {
            title: "UITextInputTraits",
            path: "/documentation/uikit/uitextinputtraits",
            note: "Customize keyboard and autocorrection behavior.",
        },
    ];

    let mut map = HashMap::new();
    map.insert(
        "swiftui::searchable(text:placement:prompt:)",
        Entry {
            quick_tip: Some("Pair with searchSuggestions(_:) and searchScopes(_:scopes:) to cover completions and scoped results."),
            related: &SWIFTUI_SEARCHABLE_RELATED,
            integration: &SWIFTUI_SEARCHABLE_INTEGRATIONS,
            snippet: Some(SWIFTUI_SEARCHABLE_SNIPPET),
        },
    );
    map.insert(
        "swiftui::textfield",
        Entry {
            quick_tip: Some(
                "Use modifiers like focused(_:equals:) to drive validation and submit actions.",
            ),
            related: &SWIFTUI_TEXTFIELD_RELATED,
            integration: &SWIFTUI_TEXTFIELD_INTEGRATIONS,
            snippet: Some(SWIFTUI_TEXTFIELD_SNIPPET),
        },
    );
    map.insert(
        "swiftui::list",
        Entry {
            quick_tip: Some("Adopt listStyle(_:) to align visuals with platform conventions."),
            related: &SWIFTUI_LIST_RELATED,
            integration: &SWIFTUI_LIST_INTEGRATIONS,
            snippet: Some(SWIFTUI_LIST_SNIPPET),
        },
    );
    map.insert(
        "swiftui::accessibilitylabel(_:)",
        Entry {
            quick_tip: Some("Combine with accessibilityHint(_:) to clarify the control’s result."),
            related: &SWIFTUI_ACCESSIBILITY_RELATED,
            integration: &[],
            snippet: Some(SWIFTUI_ACCESSIBILITY_SNIPPET),
        },
    );
    map.insert(
        "swiftui::text",
        Entry {
            quick_tip: Some("Prefer system fonts and styles for automatic Dynamic Type support."),
            related: &[
                Related {
                    title: "font(_:)",
                    path: "/documentation/swiftui/view/font(_:)",
                    note: "Apply semantic styles that adapt across platforms.",
                },
                Related {
                    title: "foregroundStyle(_:)",
                    path: "/documentation/swiftui/view/foregroundstyle(_:)",
                    note: "Use SF Symbols colors and gradients for emphasis.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UILabel",
                    path: "/documentation/uikit/uilabel",
                    note: "Use when embedding SwiftUI text inside UIKit layouts.",
                },
                Link {
                    framework: "AppKit",
                    title: "NSTextField",
                    path: "/documentation/appkit/nstextfield",
                    note: "Bridge to macOS text controls in hybrid interfaces.",
                },
            ],
            snippet: Some(SWIFTUI_TEXT_SNIPPET),
        },
    );
    map.insert(
        "swiftui::search",
        Entry {
            quick_tip: Some("Use searchSuggestions(_:), searchScopes(_:scopes:), and tokens to shape the experience."),
            related: &SWIFTUI_SEARCH_TOPIC_RELATED,
            integration: &SWIFTUI_SEARCH_TOPIC_INTEGRATIONS,
            snippet: Some(SWIFTUI_SEARCH_TOPIC_SNIPPET),
        },
    );
    map.insert(
        "uikit::uitextfield",
        Entry {
            quick_tip: Some("Adopt UITextFieldDelegate for validation and formatting."),
            related: &UIKIT_UITEXTFIELD_RELATED,
            integration: &UIKIT_UITEXTFIELD_INTEGRATIONS,
            snippet: Some(UIKIT_UITEXTFIELD_SNIPPET),
        },
    );

    // Additional SwiftUI entries
    map.insert(
        "swiftui::navigationstack",
        Entry {
            quick_tip: Some("Use NavigationStack for value-based navigation with type-safe destinations."),
            related: &[
                Related {
                    title: "NavigationLink",
                    path: "/documentation/swiftui/navigationlink",
                    note: "Create links that push views onto the stack.",
                },
                Related {
                    title: "navigationDestination(for:destination:)",
                    path: "/documentation/swiftui/view/navigationdestination(for:destination:)",
                    note: "Define destinations for value-based navigation.",
                },
                Related {
                    title: "NavigationPath",
                    path: "/documentation/swiftui/navigationpath",
                    note: "Store navigation state for programmatic control.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UINavigationController",
                    path: "/documentation/uikit/uinavigationcontroller",
                    note: "Use when embedding SwiftUI in UIKit navigation hierarchies.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var path = NavigationPath()\n\nNavigationStack(path: $path) {\n    List(items) { item in\n        NavigationLink(value: item) {\n            Text(item.title)\n        }\n    }\n    .navigationDestination(for: Item.self) { item in\n        DetailView(item: item)\n    }\n}",
                caption: Some("Programmatic navigation with type-safe destinations."),
            }),
        },
    );

    map.insert(
        "swiftui::tabview",
        Entry {
            quick_tip: Some("Use TabView with selection binding for programmatic tab switching."),
            related: &[
                Related {
                    title: "tabItem(_:)",
                    path: "/documentation/swiftui/view/tabitem(_:)",
                    note: "Configure the tab bar item for each tab.",
                },
                Related {
                    title: "badge(_:)",
                    path: "/documentation/swiftui/view/badge(_:)",
                    note: "Add notification badges to tab items.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UITabBarController",
                    path: "/documentation/uikit/uitabbarcontroller",
                    note: "Use for UIKit-based tab navigation.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var selectedTab = 0\n\nTabView(selection: $selectedTab) {\n    HomeView()\n        .tabItem { Label(\"Home\", systemImage: \"house\") }\n        .tag(0)\n    SettingsView()\n        .tabItem { Label(\"Settings\", systemImage: \"gear\") }\n        .tag(1)\n}",
                caption: Some("Tab-based navigation with programmatic selection."),
            }),
        },
    );

    map.insert(
        "swiftui::picker",
        Entry {
            quick_tip: Some("Choose picker style based on context: wheel for dates, menu for compact options."),
            related: &[
                Related {
                    title: "pickerStyle(_:)",
                    path: "/documentation/swiftui/view/pickerstyle(_:)",
                    note: "Customize picker appearance: menu, wheel, segmented, inline.",
                },
                Related {
                    title: "DatePicker",
                    path: "/documentation/swiftui/datepicker",
                    note: "Specialized picker for date and time selection.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIPickerView",
                    path: "/documentation/uikit/uipickerview",
                    note: "UIKit equivalent for wheel-style pickers.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var selection = \"Red\"\nlet colors = [\"Red\", \"Green\", \"Blue\"]\n\nPicker(\"Color\", selection: $selection) {\n    ForEach(colors, id: \\.self) { color in\n        Text(color).tag(color)\n    }\n}\n.pickerStyle(.menu)",
                caption: Some("Menu-style picker for compact selection."),
            }),
        },
    );

    map.insert(
        "swiftui::sheet",
        Entry {
            quick_tip: Some("Use sheet for modal presentations, fullScreenCover for immersive experiences."),
            related: &[
                Related {
                    title: "presentationDetents(_:)",
                    path: "/documentation/swiftui/view/presentationdetents(_:)",
                    note: "Control sheet height with medium, large, or custom detents.",
                },
                Related {
                    title: "interactiveDismissDisabled(_:)",
                    path: "/documentation/swiftui/view/interactivedismissdisabled(_:)",
                    note: "Prevent accidental dismiss during important tasks.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UISheetPresentationController",
                    path: "/documentation/uikit/uisheetpresentationcontroller",
                    note: "UIKit sheet with detent support.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var showSettings = false\n\nButton(\"Settings\") { showSettings = true }\n.sheet(isPresented: $showSettings) {\n    SettingsView()\n        .presentationDetents([.medium, .large])\n        .presentationDragIndicator(.visible)\n}",
                caption: Some("Present a sheet with multiple height options."),
            }),
        },
    );

    map.insert(
        "swiftui::asyncimage",
        Entry {
            quick_tip: Some("Always provide placeholder and error states for network images."),
            related: &[
                Related {
                    title: "Image",
                    path: "/documentation/swiftui/image",
                    note: "Use for local assets and SF Symbols.",
                },
                Related {
                    title: "resizable()",
                    path: "/documentation/swiftui/image/resizable(capinsets:resizingmode:)",
                    note: "Make images resizable before applying frame modifiers.",
                },
            ],
            integration: &[],
            snippet: Some(Snippet {
                language: "swift",
                code: "AsyncImage(url: imageURL) { phase in\n    switch phase {\n    case .empty:\n        ProgressView()\n    case .success(let image):\n        image.resizable().aspectRatio(contentMode: .fit)\n    case .failure:\n        Image(systemName: \"photo\")\n            .foregroundStyle(.secondary)\n    @unknown default:\n        EmptyView()\n    }\n}",
                caption: Some("Handle all loading states for remote images."),
            }),
        },
    );

    map.insert(
        "swiftui::progressview",
        Entry {
            quick_tip: Some("Use determinate progress for known durations, indeterminate for unknown."),
            related: &[
                Related {
                    title: "progressViewStyle(_:)",
                    path: "/documentation/swiftui/view/progressviewstyle(_:)",
                    note: "Choose linear or circular styles.",
                },
                Related {
                    title: "Gauge",
                    path: "/documentation/swiftui/gauge",
                    note: "Display values within a range with more styling options.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIProgressView",
                    path: "/documentation/uikit/uiprogressview",
                    note: "UIKit progress bar.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var progress = 0.5\n\nProgressView(value: progress) {\n    Text(\"Downloading...\")\n} currentValueLabel: {\n    Text(\"\\(Int(progress * 100))%\")\n}",
                caption: Some("Determinate progress with labels."),
            }),
        },
    );

    map.insert(
        "swiftui::form",
        Entry {
            quick_tip: Some("Use Form for settings screens; it adapts styling per platform."),
            related: &[
                Related {
                    title: "Section",
                    path: "/documentation/swiftui/section",
                    note: "Group related form controls with headers and footers.",
                },
                Related {
                    title: "LabeledContent",
                    path: "/documentation/swiftui/labeledcontent",
                    note: "Display read-only information in form rows.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UITableView",
                    path: "/documentation/uikit/uitableview",
                    note: "UIKit grouped table style for settings.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "Form {\n    Section(\"Account\") {\n        TextField(\"Username\", text: $username)\n        SecureField(\"Password\", text: $password)\n    }\n    Section(\"Preferences\") {\n        Toggle(\"Notifications\", isOn: $notifications)\n        Picker(\"Theme\", selection: $theme) {\n            Text(\"Light\").tag(0)\n            Text(\"Dark\").tag(1)\n        }\n    }\n}",
                caption: Some("Settings form with sections and controls."),
            }),
        },
    );

    map.insert(
        "swiftui::alert",
        Entry {
            quick_tip: Some("Use confirmationDialog for destructive actions, alert for informational messages."),
            related: &[
                Related {
                    title: "confirmationDialog(_:isPresented:titleVisibility:actions:message:)",
                    path: "/documentation/swiftui/view/confirmationdialog(_:ispresented:titlevisibility:actions:message:)",
                    note: "Action sheet style for destructive operations.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIAlertController",
                    path: "/documentation/uikit/uialertcontroller",
                    note: "UIKit alert and action sheet presentations.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var showAlert = false\n\nButton(\"Delete\") { showAlert = true }\n.alert(\"Delete Item?\", isPresented: $showAlert) {\n    Button(\"Cancel\", role: .cancel) { }\n    Button(\"Delete\", role: .destructive) {\n        deleteItem()\n    }\n} message: {\n    Text(\"This action cannot be undone.\")\n}",
                caption: Some("Destructive action confirmation alert."),
            }),
        },
    );

    map.insert(
        "swiftui::gesture",
        Entry {
            quick_tip: Some("Combine gestures with simultaneousGesture or sequenced for complex interactions."),
            related: &[
                Related {
                    title: "DragGesture",
                    path: "/documentation/swiftui/draggesture",
                    note: "Track drag position and velocity.",
                },
                Related {
                    title: "MagnificationGesture",
                    path: "/documentation/swiftui/magnificationgesture",
                    note: "Handle pinch-to-zoom interactions.",
                },
                Related {
                    title: "RotationGesture",
                    path: "/documentation/swiftui/rotationgesture",
                    note: "Track two-finger rotation.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIGestureRecognizer",
                    path: "/documentation/uikit/uigesturerecognizer",
                    note: "UIKit gesture recognizer base class.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var offset = CGSize.zero\n\nCircle()\n    .fill(.blue)\n    .frame(width: 100, height: 100)\n    .offset(offset)\n    .gesture(\n        DragGesture()\n            .onChanged { value in\n                offset = value.translation\n            }\n            .onEnded { _ in\n                withAnimation { offset = .zero }\n            }\n    )",
                caption: Some("Draggable view with spring-back animation."),
            }),
        },
    );

    map.insert(
        "swiftui::animation",
        Entry {
            quick_tip: Some("Use withAnimation for state changes, animation modifier for view-specific timing."),
            related: &[
                Related {
                    title: "withAnimation(_:_:)",
                    path: "/documentation/swiftui/withanimation(_:_:)",
                    note: "Animate state changes with a timing curve.",
                },
                Related {
                    title: "transition(_:)",
                    path: "/documentation/swiftui/view/transition(_:)",
                    note: "Customize how views appear and disappear.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIView.animate",
                    path: "/documentation/uikit/uiview/1622418-animate",
                    note: "UIKit block-based animations.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "@State private var isExpanded = false\n\nVStack {\n    Button(\"Toggle\") {\n        withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {\n            isExpanded.toggle()\n        }\n    }\n    if isExpanded {\n        Text(\"Expanded content\")\n            .transition(.move(edge: .top).combined(with: .opacity))\n    }\n}",
                caption: Some("Spring animation with combined transitions."),
            }),
        },
    );

    map.insert(
        "swiftui::menu",
        Entry {
            quick_tip: Some("Use Menu for compact action lists, contextMenu for long-press actions."),
            related: &[
                Related {
                    title: "contextMenu(menuItems:)",
                    path: "/documentation/swiftui/view/contextmenu(menuitems:)",
                    note: "Add long-press menu to any view.",
                },
                Related {
                    title: "Button",
                    path: "/documentation/swiftui/button",
                    note: "Menu items are buttons with optional roles.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UIMenu",
                    path: "/documentation/uikit/uimenu",
                    note: "UIKit hierarchical menu system.",
                },
            ],
            snippet: Some(Snippet {
                language: "swift",
                code: "Menu(\"Actions\") {\n    Button(\"Copy\", action: copyItem)\n    Button(\"Share\", action: shareItem)\n    Divider()\n    Button(\"Delete\", role: .destructive, action: deleteItem)\n}",
                caption: Some("Dropdown menu with destructive action."),
            }),
        },
    );

    map.insert(
        "swiftui::observable",
        Entry {
            quick_tip: Some("Use @Observable macro for simple state; ObservableObject for complex dependencies."),
            related: &[
                Related {
                    title: "@State",
                    path: "/documentation/swiftui/state",
                    note: "Local view state for simple values.",
                },
                Related {
                    title: "@Bindable",
                    path: "/documentation/swiftui/bindable",
                    note: "Create bindings to Observable properties.",
                },
                Related {
                    title: "@Environment",
                    path: "/documentation/swiftui/environment",
                    note: "Access shared data through the environment.",
                },
            ],
            integration: &[],
            snippet: Some(Snippet {
                language: "swift",
                code: "@Observable\nclass UserSettings {\n    var username = \"\"\n    var notificationsEnabled = true\n}\n\nstruct SettingsView: View {\n    @Bindable var settings: UserSettings\n    \n    var body: some View {\n        Form {\n            TextField(\"Username\", text: $settings.username)\n            Toggle(\"Notifications\", isOn: $settings.notificationsEnabled)\n        }\n    }\n}",
                caption: Some("Observable model with bindable properties."),
            }),
        },
    );

    map
});

static RECIPES: Lazy<Vec<RecipeDefinition>> = Lazy::new(|| {
    use RecipeDefinition as Recipe;
    use RelatedItem as Related;

    const SEARCH_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "Adding a search interface to your app",
            path: "/documentation/swiftui/adding-a-search-interface-to-your-app",
            note: "High-level walkthrough of the searchable modifier.",
        },
        Related {
            title: "searchable(text:placement:prompt:)",
            path: "/documentation/swiftui/view/searchable(text:placement:prompt:)",
            note: "Primary modifier for enabling search.",
        },
        Related {
            title: "List",
            path: "/documentation/swiftui/list",
            note: "Show search results within scrolling content.",
        },
    ];

    const SUGGESTIONS_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Provide completions underneath the search field.",
        },
        Related {
            title: "searchCompletion(_:)",
            path: "/documentation/swiftui/text/searchcompletion(_:)",
            note: "Associate display names with underlying values.",
        },
        Related {
            title: "searchable(text:tokens:suggestedTokens:placement:prompt:token:)",
            path: "/documentation/swiftui/view/searchable(text:tokens:suggestedtokens:placement:prompt:token:)",
            note: "Use token-based suggestions for structured filtering.",
        },
    ];

    const SCOPES_RECIPE_REFS: [Related; 2] = [
        Related {
            title: "searchScopes(_:scopes:)",
            path: "/documentation/swiftui/view/searchscopes(_:scopes:)",
            note: "Switch between categories in the search UI.",
        },
        Related {
            title: "SearchScope",
            path: "/documentation/swiftui/searchscope",
            note: "Define reusable scope identifiers.",
        },
    ];

    // SwiftUI Navigation references
    const NAVIGATION_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "NavigationStack",
            path: "/documentation/swiftui/navigationstack",
            note: "Container for push-based navigation.",
        },
        Related {
            title: "navigationDestination(for:destination:)",
            path: "/documentation/swiftui/view/navigationdestination(for:destination:)",
            note: "Register destinations for value-based navigation.",
        },
        Related {
            title: "NavigationPath",
            path: "/documentation/swiftui/navigationpath",
            note: "Type-erased path for programmatic navigation.",
        },
    ];

    // SwiftUI Sheet references
    const SHEET_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "sheet(isPresented:onDismiss:content:)",
            path: "/documentation/swiftui/view/sheet(ispresented:ondismiss:content:)",
            note: "Present a modal sheet.",
        },
        Related {
            title: "presentationDetents(_:)",
            path: "/documentation/swiftui/view/presentationdetents(_:)",
            note: "Control sheet height options.",
        },
        Related {
            title: "fullScreenCover(isPresented:onDismiss:content:)",
            path: "/documentation/swiftui/view/fullscreencover(ispresented:ondismiss:content:)",
            note: "Present a full-screen modal.",
        },
    ];

    // SwiftUI Data Flow references
    const DATA_FLOW_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "@Observable",
            path: "/documentation/observation/observable()",
            note: "Macro for creating observable models.",
        },
        Related {
            title: "@State",
            path: "/documentation/swiftui/state",
            note: "Local view state for value types.",
        },
        Related {
            title: "@Environment",
            path: "/documentation/swiftui/environment",
            note: "Access values from the environment.",
        },
    ];

    // Foundation Models references
    const FM_SESSION_REFS: [Related; 3] = [
        Related {
            title: "LanguageModelSession",
            path: "/documentation/foundationmodels/languagemodelsession",
            note: "Session for interacting with the language model.",
        },
        Related {
            title: "SystemLanguageModel",
            path: "/documentation/foundationmodels/systemlanguagemodel",
            note: "Access the on-device language model.",
        },
        Related {
            title: "GenerationOptions",
            path: "/documentation/foundationmodels/generationoptions",
            note: "Configure generation parameters.",
        },
    ];

    const FM_STRUCTURED_REFS: [Related; 3] = [
        Related {
            title: "Generable",
            path: "/documentation/foundationmodels/generable",
            note: "Protocol for structured output types.",
        },
        Related {
            title: "@Generable",
            path: "/documentation/foundationmodels/generable()",
            note: "Macro to make types generable.",
        },
        Related {
            title: "GenerationOptions",
            path: "/documentation/foundationmodels/generationoptions",
            note: "Control generation behavior.",
        },
    ];

    const FM_TOOLS_REFS: [Related; 3] = [
        Related {
            title: "Tool",
            path: "/documentation/foundationmodels/tool",
            note: "Protocol for defining callable tools.",
        },
        Related {
            title: "ToolOutput",
            path: "/documentation/foundationmodels/tooloutput",
            note: "Result type for tool execution.",
        },
        Related {
            title: "LanguageModelSession",
            path: "/documentation/foundationmodels/languagemodelsession",
            note: "Session that orchestrates tool calls.",
        },
    ];

    vec![
        Recipe {
            id: "swiftui-search-list",
            technology: "swiftui",
            title: "Add searchable support to a SwiftUI list",
            summary: "Wire a search field above a List and filter results reactively.",
            keywords: &[
                "how do i add search",
                "search list",
                "searchable list",
                "swiftui search list",
            ],
            steps: &[
                "Wrap your data in @State or @Observable so you can filter in-place.",
                "Add searchable(text:placement:prompt:) to the container that hosts the List.",
                "Filter the list items using the bound search text, ideally in a computed property.",
                "Provide empty-state content when the filtered results are empty.",
            ],
            references: &SEARCH_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-search-suggestions",
            technology: "swiftui",
            title: "Offer dynamic search suggestions",
            summary: "Display inline completions that users can tap to complete their search query.",
            keywords: &[
                "how do i add search suggestions",
                "search suggestions",
                "swifui suggestions",
                "searchcompletion",
                "search tokens",
            ],
            steps: &[
                "Maintain a lightweight suggestions array derived from recent searches or server hints.",
                "Call searchSuggestions(_:) within the searchable modifier to render completions.",
                "Use searchCompletion(_:) or tokens to map suggestion taps to structured values.",
                "Update the suggestions array in onChange(of:) to keep results relevant.",
            ],
            references: &SUGGESTIONS_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-search-scopes",
            technology: "swiftui",
            title: "Limit search results with scopes",
            summary: "Add segmented controls that keep the search query while filtering categories.",
            keywords: &[
                "how do i add search scope",
                "search scope",
                "searchScopes",
                "scope search",
            ],
            steps: &[
                "Define an enum that conforms to Hashable to represent each scope.",
                "Bind the selected scope to state and update the search predicate accordingly.",
                "Add searchScopes(_:scopes:) to describe the available categories.",
                "Adjust the search results view to react to both the text query and the selected scope.",
            ],
            references: &SCOPES_RECIPE_REFS,
        },
        // New SwiftUI recipes
        Recipe {
            id: "swiftui-navigation-stack",
            technology: "swiftui",
            title: "Set up NavigationStack with value-based navigation",
            summary: "Create type-safe, programmatic navigation using NavigationStack and NavigationPath.",
            keywords: &[
                "how do i add navigation",
                "create navigation",
                "navigationstack",
                "navigation stack",
                "push view",
                "programmatic navigation",
            ],
            steps: &[
                "Create a NavigationStack as the root of your navigation hierarchy.",
                "Define your data models that will drive navigation (they must be Hashable).",
                "Use NavigationLink(value:) to create links that push values onto the stack.",
                "Register destinations with navigationDestination(for:destination:) for each type.",
                "Optionally bind a NavigationPath to @State for programmatic navigation control.",
            ],
            references: &NAVIGATION_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-sheet-modal",
            technology: "swiftui",
            title: "Present a modal sheet with detents",
            summary: "Show a sheet that can resize between medium and large heights.",
            keywords: &[
                "how do i show sheet",
                "present sheet",
                "modal sheet",
                "bottom sheet",
                "sheet detents",
                "half sheet",
            ],
            steps: &[
                "Create a @State Bool to control the sheet's presentation.",
                "Attach .sheet(isPresented:content:) to a view in your hierarchy.",
                "Inside the sheet content, apply .presentationDetents([.medium, .large]) for resizable heights.",
                "Optionally add .presentationDragIndicator(.visible) for a grab handle.",
                "Use .interactiveDismissDisabled() if you need to prevent swipe-to-dismiss.",
            ],
            references: &SHEET_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-observable-data",
            technology: "swiftui",
            title: "Use @Observable for reactive data models",
            summary: "Create observable models that automatically update views when properties change.",
            keywords: &[
                "how do i use observable",
                "observable model",
                "data model",
                "state management",
                "reactive data",
                "bindable",
            ],
            steps: &[
                "Mark your class with @Observable macro to enable automatic observation.",
                "Declare properties as regular var - they're automatically tracked.",
                "Pass the model to views directly or through @Environment.",
                "Use @Bindable when you need two-way bindings to observable properties.",
                "Views will automatically re-render when observed properties change.",
            ],
            references: &DATA_FLOW_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-async-image",
            technology: "swiftui",
            title: "Load remote images with AsyncImage",
            summary: "Display images from URLs with loading and error states.",
            keywords: &[
                "how do i load image",
                "remote image",
                "asyncimage",
                "async image",
                "url image",
                "network image",
            ],
            steps: &[
                "Create an AsyncImage with the URL of your remote image.",
                "Use the phase-based initializer to handle loading, success, and failure states.",
                "Show a ProgressView() during the .empty loading phase.",
                "In the .success phase, apply resizable() and aspectRatio() to the image.",
                "Provide a placeholder image for the .failure phase.",
            ],
            references: &[
                Related {
                    title: "AsyncImage",
                    path: "/documentation/swiftui/asyncimage",
                    note: "View that loads and displays remote images.",
                },
                Related {
                    title: "AsyncImagePhase",
                    path: "/documentation/swiftui/asyncimagephase",
                    note: "Represents the loading state of an async image.",
                },
            ],
        },
        Recipe {
            id: "swiftui-list-swipe",
            technology: "swiftui",
            title: "Add swipe actions to List rows",
            summary: "Enable swipe-to-delete and custom swipe actions on list items.",
            keywords: &[
                "how do i add swipe",
                "swipe actions",
                "swipe to delete",
                "list actions",
                "row actions",
            ],
            steps: &[
                "Create a List with ForEach to iterate over your data.",
                "Apply .swipeActions(edge:allowsFullSwipe:content:) to each row.",
                "Use edge: .trailing for delete actions (swipe left to reveal).",
                "Use edge: .leading for secondary actions (swipe right to reveal).",
                "Add Button views with appropriate roles (.destructive for delete).",
            ],
            references: &[
                Related {
                    title: "swipeActions(edge:allowsFullSwipe:content:)",
                    path: "/documentation/swiftui/view/swipeactions(edge:allowsfullswipe:content:)",
                    note: "Add swipe actions to list rows.",
                },
                Related {
                    title: "List",
                    path: "/documentation/swiftui/list",
                    note: "Container for displaying rows of data.",
                },
            ],
        },
        // Foundation Models recipes
        Recipe {
            id: "fm-create-session",
            technology: "foundation models",
            title: "Create a language model session",
            summary: "Initialize a session with the on-device language model for text generation.",
            keywords: &[
                "how do i create session",
                "language model session",
                "create session",
                "start session",
                "foundation models session",
            ],
            steps: &[
                "Check model availability with SystemLanguageModel.default.availability.",
                "Handle unavailable cases: .deviceNotEligible, .appleIntelligenceNotEnabled, .modelNotReady.",
                "Create a LanguageModelSession with the default model when available.",
                "Use session.respond(to:) to generate responses to prompts.",
                "Handle the async response, which may be streamed or complete.",
            ],
            references: &FM_SESSION_REFS,
        },
        Recipe {
            id: "fm-structured-output",
            technology: "foundation models",
            title: "Generate structured output with @Generable",
            summary: "Get type-safe responses from the model using custom Swift types.",
            keywords: &[
                "how do i get structured output",
                "structured output",
                "generable",
                "typed response",
                "json output",
                "parse response",
            ],
            steps: &[
                "Define a struct and mark it with @Generable macro.",
                "Include properties for each piece of data you want extracted.",
                "Add descriptions to properties using @Guide for better results.",
                "Use session.respond(to:generating:) with your Generable type.",
                "Access the typed result directly from the response.",
            ],
            references: &FM_STRUCTURED_REFS,
        },
        Recipe {
            id: "fm-tool-calling",
            technology: "foundation models",
            title: "Implement tool calling for the language model",
            summary: "Let the model invoke your functions to perform actions or retrieve data.",
            keywords: &[
                "how do i add tool",
                "tool calling",
                "function calling",
                "tools",
                "model tools",
            ],
            steps: &[
                "Define a struct conforming to Tool protocol for each capability.",
                "Implement the call() method that performs the actual work.",
                "Add parameter descriptions using @Guide for the model to understand usage.",
                "Register tools when creating the session or in respond() call.",
                "The model will invoke tools as needed and incorporate results.",
            ],
            references: &FM_TOOLS_REFS,
        },
    ]
});

pub fn lookup(technology: &str, symbol_title: &str) -> Option<&'static KnowledgeEntry> {
    let key = format!(
        "{}::{}",
        technology.trim().to_lowercase(),
        symbol_title.trim().to_lowercase()
    );
    KNOWLEDGE.get(key.as_str())
}

pub fn find_recipe(technology: &str, query: &str) -> Option<&'static RecipeDefinition> {
    RECIPES
        .iter()
        .find(|recipe| recipe.matches(query, technology))
}

pub fn recipes_for(technology: &str) -> Vec<&'static RecipeDefinition> {
    RECIPES
        .iter()
        .filter(|recipe| recipe.technology.eq_ignore_ascii_case(technology))
        .collect()
}

pub fn snippet(entry: &KnowledgeEntry) -> Option<Snippet> {
    entry.snippet
}

pub fn related_items(entry: &KnowledgeEntry) -> &'static [RelatedItem] {
    entry.related
}

pub fn integration_links(entry: &KnowledgeEntry) -> &'static [IntegrationLink] {
    entry.integration
}
const SWIFTUI_SEARCHABLE_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "List(filteredBooks) { book in\n    Text(book.title)\n}\n.searchable(text: $query, placement: .navigationBarDrawer, prompt: \"Search books\")",
        caption: Some("Attach `searchable` to filter list content reactively."),
    };
