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
}

pub async fn run(args: TopologyArgs) -> anyhow::Result<()> {
    match args.action {
        TopologyAction::Layout { graph, schema, output, width, height } => {
            cmd_layout(&graph, schema.as_deref(), &output, width, height)
        }
        TopologyAction::Validate { schema } => cmd_validate(&schema),
        TopologyAction::Detect { graph } => cmd_detect(&graph),
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
