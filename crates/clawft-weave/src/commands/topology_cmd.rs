//! `weaver topology` subcommand — layout and schema management.
//!
//! - `weaver topology layout <graph.json> --schema <schema.yaml>` — compute positioned geometry
//! - `weaver topology validate <schema.yaml>` — validate a topology schema
//! - `weaver topology detect <graph.json>` — auto-detect geometry from graph structure

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Topology layout, schema validation, and geometry detection")]
pub struct TopologyArgs {
    #[command(subcommand)]
    pub action: TopologyAction,
}

#[derive(Subcommand)]
pub enum TopologyAction {
    /// Compute positioned geometry from a knowledge graph + schema.
    Layout {
        /// Path to the knowledge graph JSON (graphify export).
        graph: PathBuf,
        /// Path to the topology schema YAML.
        #[arg(short, long)]
        schema: Option<PathBuf>,
        /// Output path for positioned geometry JSON.
        #[arg(short, long, default_value = "positioned-graph.json")]
        output: PathBuf,
        /// Viewport width.
        #[arg(long, default_value_t = 1200.0)]
        width: f64,
        /// Viewport height.
        #[arg(long, default_value_t = 800.0)]
        height: f64,
    },
    /// Validate a topology schema YAML file.
    Validate {
        /// Path to the schema YAML.
        schema: PathBuf,
    },
    /// Auto-detect the best geometry for a knowledge graph.
    Detect {
        /// Path to the knowledge graph JSON.
        graph: PathBuf,
    },
    /// Infer a topology schema from a knowledge graph.
    Infer {
        /// Path to the knowledge graph JSON.
        graph: PathBuf,
        /// Name for the inferred schema.
        #[arg(short, long, default_value = "inferred")]
        name: String,
        /// Output path for the inferred schema YAML.
        #[arg(short, long, default_value = "inferred.topology.yaml")]
        output: PathBuf,
    },
    /// Diff a declared schema against one inferred from a graph.
    Diff {
        /// Path to the declared schema YAML.
        declared: PathBuf,
        /// Path to the knowledge graph JSON to infer from.
        graph: PathBuf,
    },
    /// Generate sliced graphs for progressive drill-down navigation.
    Slice {
        /// Path to the knowledge graph JSON.
        graph: PathBuf,
        /// Path to the topology schema YAML.
        #[arg(short, long)]
        schema: Option<PathBuf>,
        /// Output directory for slice JSON files.
        #[arg(short, long, default_value = "slices")]
        output: PathBuf,
        /// Viewport width.
        #[arg(long, default_value_t = 1200.0)]
        width: f64,
        /// Viewport height.
        #[arg(long, default_value_t = 800.0)]
        height: f64,
    },
    /// Export a knowledge graph as VOWL JSON for the navigator widget.
    Vowl {
        /// Path to the knowledge graph JSON.
        graph: PathBuf,
        /// Path to the topology schema YAML.
        #[arg(short, long)]
        schema: Option<PathBuf>,
        /// Output path for VOWL JSON.
        #[arg(short, long, default_value = "vowl-graph.json")]
        output: PathBuf,
    },
}

pub async fn run(args: TopologyArgs) -> anyhow::Result<()> {
    match args.action {
        TopologyAction::Layout { graph, schema, output, width, height } => {
            cmd_layout(&graph, schema.as_deref(), &output, width, height)
        }
        TopologyAction::Validate { schema } => cmd_validate(&schema),
        TopologyAction::Detect { graph } => cmd_detect(&graph),
        TopologyAction::Slice { graph, schema, output, width, height } => {
            cmd_slice(&graph, schema.as_deref(), &output, width, height)
        }
        TopologyAction::Infer { graph, name, output } => cmd_infer(&graph, &name, &output),
        TopologyAction::Diff { declared, graph } => cmd_diff(&declared, &graph),
        TopologyAction::Vowl { graph, schema, output } => cmd_vowl(&graph, schema.as_deref(), &output),
    }
}

fn load_graph(path: &PathBuf) -> anyhow::Result<clawft_graphify::KnowledgeGraph> {
    let data = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&data)?;
    let kg = clawft_graphify::build::build_from_json(&json)?;
    Ok(kg)
}

fn load_schema(path: Option<&std::path::Path>) -> anyhow::Result<clawft_graphify::topology::TopologySchema> {
    match path {
        Some(p) => {
            let yaml = std::fs::read_to_string(p)?;
            let schema = clawft_graphify::topology::TopologySchema::from_yaml(&yaml)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(schema)
        }
        None => {
            let yaml = include_str!("../../../clawft-graphify/schemas/software.yaml");
            let schema = clawft_graphify::topology::TopologySchema::from_yaml(yaml)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(schema)
        }
    }
}

fn cmd_layout(
    graph_path: &PathBuf,
    schema_path: Option<&std::path::Path>,
    output_path: &PathBuf,
    width: f64,
    height: f64,
) -> anyhow::Result<()> {
    let kg = load_graph(graph_path)?;
    let schema = load_schema(schema_path)?;

    println!(
        "Laying out {} nodes, {} edges with schema '{}' ({:?} root geometry)",
        kg.entity_count(),
        kg.relationship_count(),
        schema.name,
        schema.modes.structure.root_geometry,
    );

    let start = std::time::Instant::now();
    let positioned = clawft_graphify::layout::layout_graph(&kg, &schema, width, height);
    let elapsed = start.elapsed();

    let json = serde_json::to_string_pretty(&positioned)?;
    std::fs::write(output_path, &json)?;

    println!(
        "Layout complete in {:.1}ms — {} nodes, {} edges → {}",
        elapsed.as_secs_f64() * 1000.0,
        positioned.nodes.len(),
        positioned.edges.len(),
        output_path.display(),
    );

    Ok(())
}

fn cmd_validate(schema_path: &PathBuf) -> anyhow::Result<()> {
    let yaml = std::fs::read_to_string(schema_path)?;
    let schema = clawft_graphify::topology::TopologySchema::from_yaml(&yaml)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let warnings = schema.validate();

    println!("Schema: {} v{}", schema.name, schema.version);
    if let Some(iri) = &schema.iri {
        println!("IRI: {iri}");
    }
    println!("Node types: {}", schema.nodes.len());
    println!("Edge types: {}", schema.edges.len());

    if warnings.is_empty() {
        println!("\nValid — no warnings.");
    } else {
        println!("\n{} warning(s):", warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }

    Ok(())
}

fn cmd_detect(graph_path: &PathBuf) -> anyhow::Result<()> {
    let kg = load_graph(graph_path)?;

    println!(
        "Graph: {} nodes, {} edges",
        kg.entity_count(),
        kg.relationship_count(),
    );

    // Count edge types.
    let mut edge_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, _, rel) in kg.edges() {
        *edge_type_counts.entry(format!("{:?}", rel.relation_type)).or_default() += 1;
    }

    println!("\nEdge type distribution:");
    let mut sorted: Vec<_> = edge_type_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (etype, count) in &sorted {
        let pct = (**count) as f64 / kg.relationship_count().max(1) as f64 * 100.0;
        println!("  {etype}: {count} ({pct:.0}%)");
    }

    // Count entity types.
    let mut entity_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for entity in kg.entities() {
        *entity_type_counts.entry(entity.entity_type.discriminant().to_string()).or_default() += 1;
    }

    println!("\nEntity type distribution:");
    let mut sorted: Vec<_> = entity_type_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (etype, count) in &sorted {
        println!("  {etype}: {count}");
    }

    // Triage classification.
    let mut atoms = 0usize;
    let mut sequences = 0usize;
    let mut branches = 0usize;
    for entity in kg.entities() {
        match clawft_graphify::layout::triage::classify(&kg, &entity.id) {
            clawft_graphify::layout::triage::TopologyForm::Atom => atoms += 1,
            clawft_graphify::layout::triage::TopologyForm::Sequence => sequences += 1,
            clawft_graphify::layout::triage::TopologyForm::Branch => branches += 1,
        }
    }

    println!("\nTree calculus triage:");
    println!("  Atom (leaf): {atoms}");
    println!("  Sequence (same-type children): {sequences}");
    println!("  Branch (mixed-type children): {branches}");

    // Auto-detect recommendation.
    let default_schema = load_schema(None)?;
    let detected = clawft_graphify::layout::detect_geometry(&kg, &default_schema);
    println!("\nRecommended geometry: {detected:?}");

    Ok(())
}

fn cmd_infer(graph_path: &PathBuf, name: &str, output_path: &PathBuf) -> anyhow::Result<()> {
    let kg = load_graph(graph_path)?;

    println!(
        "Inferring schema from {} nodes, {} edges...",
        kg.entity_count(),
        kg.relationship_count(),
    );

    let schema = clawft_graphify::topology_infer::infer_schema(&kg, name);
    let warnings = schema.validate();

    let yaml = serde_yaml::to_string(&schema)?;
    std::fs::write(output_path, &yaml)?;

    println!(
        "Inferred schema '{}': {} node types, {} edge types → {}",
        schema.name,
        schema.nodes.len() - 1, // exclude wildcard
        schema.edges.len(),
        output_path.display(),
    );

    if !warnings.is_empty() {
        println!("\n{} warning(s):", warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }

    Ok(())
}

fn cmd_diff(declared_path: &PathBuf, graph_path: &PathBuf) -> anyhow::Result<()> {
    let declared_yaml = std::fs::read_to_string(declared_path)?;
    let declared = clawft_graphify::topology::TopologySchema::from_yaml(&declared_yaml)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let kg = load_graph(graph_path)?;
    let inferred = clawft_graphify::topology_infer::infer_schema(&kg, "inferred");

    let diff = clawft_graphify::topology_infer::diff_schemas(&declared, &inferred);

    println!(
        "Schema diff: '{}' (declared) vs inferred from graph",
        declared.name,
    );

    if diff.is_empty() {
        println!("\nNo differences — declared schema matches the graph.");
        return Ok(());
    }

    if !diff.added_types.is_empty() {
        println!("\nNew entity types in graph (not in schema):");
        for t in &diff.added_types {
            println!("  + {t}");
        }
    }

    if !diff.removed_types.is_empty() {
        println!("\nDeclared types missing from graph:");
        for t in &diff.removed_types {
            println!("  - {t}");
        }
    }

    if !diff.geometry_changes.is_empty() {
        println!("\nGeometry mismatches:");
        for g in &diff.geometry_changes {
            println!("  ~ {g}");
        }
    }

    if !diff.added_edges.is_empty() {
        println!("\nNew edge types in graph:");
        for e in &diff.added_edges {
            println!("  + {e}");
        }
    }

    if !diff.removed_edges.is_empty() {
        println!("\nDeclared edge types missing from graph:");
        for e in &diff.removed_edges {
            println!("  - {e}");
        }
    }

    Ok(())
}

fn cmd_slice(
    graph_path: &PathBuf,
    schema_path: Option<&std::path::Path>,
    output_dir: &PathBuf,
    width: f64,
    height: f64,
) -> anyhow::Result<()> {
    let kg = load_graph(graph_path)?;
    let schema = load_schema(schema_path)?;

    println!(
        "Slicing {} nodes, {} edges into drill-down layers...",
        kg.entity_count(),
        kg.relationship_count(),
    );

    let manifest = clawft_graphify::layout::slicer::generate_all_slices(
        &kg, &schema, output_dir, width, height,
    ).map_err(|e| anyhow::anyhow!("{e}"))?;

    println!(
        "Generated {} slices ({} expandable nodes) → {}",
        manifest.slices.len() + 1,
        manifest.slices.len(),
        output_dir.display(),
    );
    println!("  root.json: top-level view");
    for (id, file) in &manifest.slices {
        println!("  {file}: {}", &id[..16]);
    }
    println!("  manifest.json: slice index");

    Ok(())
}

fn cmd_vowl(
    graph_path: &PathBuf,
    schema_path: Option<&std::path::Path>,
    output_path: &PathBuf,
) -> anyhow::Result<()> {
    let kg = load_graph(graph_path)?;
    let schema = load_schema(schema_path)?;

    println!(
        "Exporting {} nodes, {} edges as VOWL JSON...",
        kg.entity_count(),
        kg.relationship_count(),
    );

    let vowl = clawft_graphify::export::vowl::to_vowl_json(&kg, &schema)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let json = serde_json::to_string_pretty(&vowl)?;
    std::fs::write(output_path, &json)?;

    println!(
        "VOWL JSON: {} classes, {} properties → {}",
        vowl["metrics"]["classCount"],
        vowl["metrics"]["objectPropertyCount"],
        output_path.display(),
    );

    Ok(())
}
