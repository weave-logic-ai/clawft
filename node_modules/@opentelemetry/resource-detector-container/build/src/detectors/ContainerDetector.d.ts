import { Detector, Resource, ResourceDetectionConfig } from '@opentelemetry/resources';
export declare class ContainerDetector implements Detector {
    readonly CONTAINER_ID_LENGTH = 64;
    readonly DEFAULT_CGROUP_V1_PATH = "/proc/self/cgroup";
    readonly DEFAULT_CGROUP_V2_PATH = "/proc/self/mountinfo";
    readonly UTF8_UNICODE = "utf8";
    readonly HOSTNAME = "hostname";
    private static readFileAsync;
    detect(_config?: ResourceDetectionConfig): Promise<Resource>;
    private _getContainerIdV1;
    private _getContainerIdV2;
    private _getContainerId;
}
export declare const containerDetector: ContainerDetector;
//# sourceMappingURL=ContainerDetector.d.ts.map