/**
 * Domain-Specific Attention Configuration Examples
 *
 * This module exports real-world configurations and examples for various industries
 * and use cases, demonstrating how to adapt AgentDB's attention mechanisms for
 * specific requirements.
 */

export {
  TRADING_ATTENTION_CONFIG,
  TRADING_PERFORMANCE_TARGETS,
  TRADING_CONFIG_VARIATIONS,
  type TradingMetrics,
  type TradingSignal,
  matchTradingPattern,
  adaptConfigToMarket
} from './trading-systems';

export {
  MEDICAL_ATTENTION_CONFIG,
  MEDICAL_PERFORMANCE_TARGETS,
  MEDICAL_CONFIG_VARIATIONS,
  type MedicalMetrics,
  type SimilarCase,
  type MedicalDataQuality,
  findSimilarCases,
  adaptConfigToUrgency,
  validateMedicalData
} from './medical-imaging';

export {
  ROBOTICS_ATTENTION_CONFIG,
  ROBOTICS_PERFORMANCE_TARGETS,
  ROBOTICS_CONFIG_VARIATIONS,
  type RoboticsMetrics,
  type RobotContext,
  type NavigationPlan,
  matchEnvironment,
  adaptConfigToEnvironment,
  adaptConfigToPower
} from './robotics-navigation';

export {
  ECOMMERCE_ATTENTION_CONFIG,
  ECOMMERCE_PERFORMANCE_TARGETS,
  ECOMMERCE_CONFIG_VARIATIONS,
  type ECommerceMetrics,
  type Recommendation,
  type PromotionalContext,
  recommendProducts,
  adaptConfigToUserSegment,
  adaptConfigToPromotion,
  generateABTestConfigs
} from './e-commerce-recommendations';

export {
  RESEARCH_ATTENTION_CONFIG,
  RESEARCH_PERFORMANCE_TARGETS,
  RESEARCH_CONFIG_VARIATIONS,
  type ResearchMetrics,
  type ResearchConnection,
  type CitationNetworkMetrics,
  discoverRelatedResearch,
  adaptConfigToSearchMode,
  adaptConfigToResearchStage,
  analyzeCitationNetwork
} from './scientific-research';

export {
  IOT_ATTENTION_CONFIG,
  IOT_PERFORMANCE_TARGETS,
  IOT_CONFIG_VARIATIONS,
  type IoTMetrics,
  type Sensor,
  type AnomalyAlert,
  type NetworkTopology,
  detectAnomalies,
  adaptConfigToDeployment,
  adaptConfigToBattery,
  adaptConfigToTopology
} from './iot-sensor-networks';
