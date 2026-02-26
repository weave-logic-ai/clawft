import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useSkillsStore } from "../stores/skills-store";
import { Card, CardHeader, CardTitle, CardContent, CardFooter } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Dialog, DialogHeader, DialogTitle, DialogFooter } from "../components/ui/dialog";
import type { SkillData, RegistrySkill } from "../lib/types";

function SkillCard({
  skill,
  onUninstall,
}: {
  skill: SkillData;
  onUninstall: (name: string) => void;
}) {
  const [uninstalling, setUninstalling] = useState(false);

  const handleUninstall = async () => {
    setUninstalling(true);
    try {
      onUninstall(skill.name);
    } finally {
      setUninstalling(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle>{skill.name}</CardTitle>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              v{skill.version}
              {skill.author && <span> by {skill.author}</span>}
            </p>
          </div>
          <Badge variant={skill.enabled ? "success" : "secondary"}>
            {skill.enabled ? "enabled" : "disabled"}
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-gray-600 dark:text-gray-400">
          {skill.description}
        </p>
        <div className="mt-3 flex flex-wrap gap-1">
          {skill.tags.map((tag) => (
            <Badge key={tag} variant="outline" className="text-xs">
              {tag}
            </Badge>
          ))}
        </div>
      </CardContent>
      <CardFooter>
        <Button
          variant="destructive"
          size="sm"
          onClick={handleUninstall}
          disabled={uninstalling}
        >
          {uninstalling ? "Removing..." : "Uninstall"}
        </Button>
      </CardFooter>
    </Card>
  );
}

function RegistrySkillRow({
  skill,
  onInstall,
}: {
  skill: RegistrySkill;
  onInstall: (id: string) => void;
}) {
  const [installing, setInstalling] = useState(false);

  const handleInstall = async () => {
    setInstalling(true);
    try {
      onInstall(skill.id);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="flex items-center justify-between border-b border-gray-100 py-3 last:border-0 dark:border-gray-700">
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {skill.name}
          </span>
          <span className="text-xs text-gray-500">v{skill.version}</span>
          {skill.signed && (
            <Badge variant="success" className="text-xs">
              signed
            </Badge>
          )}
        </div>
        <p className="mt-0.5 text-sm text-gray-500 dark:text-gray-400">
          {skill.description}
        </p>
        <div className="mt-1 flex items-center gap-2 text-xs text-gray-400">
          <span>{skill.author}</span>
          <span>&#9733; {skill.stars}</span>
        </div>
      </div>
      <Button
        variant="default"
        size="sm"
        onClick={handleInstall}
        disabled={installing}
      >
        {installing ? "Installing..." : "Install"}
      </Button>
    </div>
  );
}

function SkillSkeletonCard() {
  return (
    <Card>
      <CardHeader>
        <Skeleton className="h-5 w-32" />
        <Skeleton className="mt-1 h-3 w-48" />
      </CardHeader>
      <CardContent>
        <Skeleton className="h-4 w-full" />
        <Skeleton className="mt-2 h-4 w-3/4" />
      </CardContent>
      <CardFooter>
        <Skeleton className="h-8 w-20" />
      </CardFooter>
    </Card>
  );
}

export function SkillsPage() {
  const {
    skills,
    registryResults,
    loading,
    searchQuery,
    registryOpen,
    setSearchQuery,
    setRegistryOpen,
    fetchSkills,
    searchRegistry,
    installSkill,
    uninstallSkill,
  } = useSkillsStore();

  const [registryQuery, setRegistryQuery] = useState("");
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    fetchSkills();
  }, [fetchSkills]);

  const handleSearchChange = useCallback(
    (value: string) => {
      setSearchQuery(value);
    },
    [setSearchQuery],
  );

  const handleRegistrySearch = useCallback(
    (value: string) => {
      setRegistryQuery(value);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        if (value.trim()) {
          searchRegistry(value);
        }
      }, 300);
    },
    [searchRegistry],
  );

  const filteredSkills = useMemo(() => {
    if (!searchQuery) return skills;
    const q = searchQuery.toLowerCase();
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.tags.some((t) => t.toLowerCase().includes(q)),
    );
  }, [skills, searchQuery]);

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Skills
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Browse and manage installed skills
          </p>
        </div>
        <div className="flex items-center gap-3">
          {skills.length > 0 && (
            <Badge variant="secondary">{skills.length} installed</Badge>
          )}
          <Button
            variant="default"
            size="sm"
            onClick={() => setRegistryOpen(true)}
          >
            Browse Registry
          </Button>
        </div>
      </div>

      {/* Search */}
      <div>
        <input
          type="text"
          placeholder="Search installed skills..."
          value={searchQuery}
          onChange={(e) => handleSearchChange(e.target.value)}
          className="w-full max-w-sm rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
        />
      </div>

      {/* Grid */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        {loading
          ? Array.from({ length: 6 }).map((_, i) => (
              <SkillSkeletonCard key={i} />
            ))
          : filteredSkills.map((skill) => (
              <SkillCard
                key={skill.name}
                skill={skill}
                onUninstall={uninstallSkill}
              />
            ))}
      </div>

      {!loading && filteredSkills.length === 0 && (
        <div className="py-12 text-center">
          <p className="text-gray-500 dark:text-gray-400">
            {searchQuery
              ? "No skills match your search."
              : "No skills installed yet. Browse the registry to get started."}
          </p>
        </div>
      )}

      {/* Registry Dialog */}
      <Dialog open={registryOpen} onClose={() => setRegistryOpen(false)} className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Browse ClawHub Registry</DialogTitle>
        </DialogHeader>

        <input
          type="text"
          placeholder="Search registry..."
          value={registryQuery}
          onChange={(e) => handleRegistrySearch(e.target.value)}
          className="mb-4 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
          autoFocus
        />

        <div className="max-h-80 overflow-auto">
          {registryResults.length > 0 ? (
            registryResults.map((skill) => (
              <RegistrySkillRow
                key={skill.id}
                skill={skill}
                onInstall={installSkill}
              />
            ))
          ) : (
            <p className="py-8 text-center text-sm text-gray-500 dark:text-gray-400">
              {registryQuery
                ? "No results found."
                : "Type to search the registry."}
            </p>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setRegistryOpen(false)}
          >
            Close
          </Button>
        </DialogFooter>
      </Dialog>
    </div>
  );
}
