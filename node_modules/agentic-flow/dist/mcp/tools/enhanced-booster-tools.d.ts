/**
 * Enhanced Agent Booster MCP Tools
 *
 * RuVector-powered code editing with:
 * - SONA learning (0.05ms updates)
 * - HNSW cache (150x faster pattern recall)
 * - GNN matching (differentiable search)
 * - Confidence improvement through learning
 */
/**
 * MCP Tool definitions for enhanced booster v2
 */
export declare const enhancedBoosterTools: ({
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            code: {
                type: string;
                description: string;
            };
            edit: {
                type: string;
                description: string;
            };
            language: {
                type: string;
                description: string;
            };
            filePath: {
                type: string;
                description: string;
            };
            target_filepath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            target_filepath: {
                type: string;
                description: string;
            };
            edit: {
                type: string;
                description: string;
            };
            language: {
                type: string;
                description: string;
            };
            code?: undefined;
            filePath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            filePath?: undefined;
            target_filepath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: any[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            iterations: {
                type: string;
                description: string;
                default: number;
            };
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            filePath?: undefined;
            target_filepath?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: any[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            patternId: {
                type: string;
                description: string;
            };
            success: {
                type: string;
                description: string;
            };
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            filePath?: undefined;
            target_filepath?: undefined;
            iterations?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            edits: {
                type: string;
                description: string;
                items: {
                    type: string;
                    properties: {
                        code: {
                            type: string;
                        };
                        edit: {
                            type: string;
                        };
                        language: {
                            type: string;
                        };
                        filePath: {
                            type: string;
                        };
                    };
                    required: string[];
                };
            };
            maxConcurrency: {
                type: string;
                description: string;
                default: number;
            };
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            filePath?: undefined;
            target_filepath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            topK?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            filePath: {
                type: string;
                description: string;
            };
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            target_filepath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
            topK?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            filePath: {
                type: string;
                description: string;
            };
            topK: {
                type: string;
                description: string;
                default: number;
            };
            code?: undefined;
            edit?: undefined;
            language?: undefined;
            target_filepath?: undefined;
            iterations?: undefined;
            patternId?: undefined;
            success?: undefined;
            edits?: undefined;
            maxConcurrency?: undefined;
        };
        required: string[];
    };
})[];
/**
 * MCP Tool handlers
 */
export declare const enhancedBoosterHandlers: {
    enhanced_booster_edit: (params: {
        code: string;
        edit: string;
        language: string;
        filePath?: string;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_edit_file: (params: {
        target_filepath: string;
        edit: string;
        language?: string;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
        isError: boolean;
    } | {
        content: {
            type: string;
            text: string;
        }[];
        isError?: undefined;
    }>;
    enhanced_booster_stats: () => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_pretrain: () => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_benchmark: (params: {
        iterations?: number;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_record_outcome: (params: {
        patternId: string;
        success: boolean;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_batch: (params: {
        edits: Array<{
            code: string;
            edit: string;
            language: string;
            filePath?: string;
        }>;
        maxConcurrency?: number;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_prefetch: (params: {
        filePath: string;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
    enhanced_booster_likely_files: (params: {
        filePath: string;
        topK?: number;
    }) => Promise<{
        content: {
            type: string;
            text: string;
        }[];
    }>;
};
declare const _default: {
    tools: ({
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                code: {
                    type: string;
                    description: string;
                };
                edit: {
                    type: string;
                    description: string;
                };
                language: {
                    type: string;
                    description: string;
                };
                filePath: {
                    type: string;
                    description: string;
                };
                target_filepath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: string[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                target_filepath: {
                    type: string;
                    description: string;
                };
                edit: {
                    type: string;
                    description: string;
                };
                language: {
                    type: string;
                    description: string;
                };
                code?: undefined;
                filePath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: string[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                filePath?: undefined;
                target_filepath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: any[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                iterations: {
                    type: string;
                    description: string;
                    default: number;
                };
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                filePath?: undefined;
                target_filepath?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: any[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                patternId: {
                    type: string;
                    description: string;
                };
                success: {
                    type: string;
                    description: string;
                };
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                filePath?: undefined;
                target_filepath?: undefined;
                iterations?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: string[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                edits: {
                    type: string;
                    description: string;
                    items: {
                        type: string;
                        properties: {
                            code: {
                                type: string;
                            };
                            edit: {
                                type: string;
                            };
                            language: {
                                type: string;
                            };
                            filePath: {
                                type: string;
                            };
                        };
                        required: string[];
                    };
                };
                maxConcurrency: {
                    type: string;
                    description: string;
                    default: number;
                };
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                filePath?: undefined;
                target_filepath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                topK?: undefined;
            };
            required: string[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                filePath: {
                    type: string;
                    description: string;
                };
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                target_filepath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
                topK?: undefined;
            };
            required: string[];
        };
    } | {
        name: string;
        description: string;
        inputSchema: {
            type: string;
            properties: {
                filePath: {
                    type: string;
                    description: string;
                };
                topK: {
                    type: string;
                    description: string;
                    default: number;
                };
                code?: undefined;
                edit?: undefined;
                language?: undefined;
                target_filepath?: undefined;
                iterations?: undefined;
                patternId?: undefined;
                success?: undefined;
                edits?: undefined;
                maxConcurrency?: undefined;
            };
            required: string[];
        };
    })[];
    handlers: {
        enhanced_booster_edit: (params: {
            code: string;
            edit: string;
            language: string;
            filePath?: string;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_edit_file: (params: {
            target_filepath: string;
            edit: string;
            language?: string;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
            isError: boolean;
        } | {
            content: {
                type: string;
                text: string;
            }[];
            isError?: undefined;
        }>;
        enhanced_booster_stats: () => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_pretrain: () => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_benchmark: (params: {
            iterations?: number;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_record_outcome: (params: {
            patternId: string;
            success: boolean;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_batch: (params: {
            edits: Array<{
                code: string;
                edit: string;
                language: string;
                filePath?: string;
            }>;
            maxConcurrency?: number;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_prefetch: (params: {
            filePath: string;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
        enhanced_booster_likely_files: (params: {
            filePath: string;
            topK?: number;
        }) => Promise<{
            content: {
                type: string;
                text: string;
            }[];
        }>;
    };
};
export default _default;
//# sourceMappingURL=enhanced-booster-tools.d.ts.map