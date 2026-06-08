import governance from '../../guides/governance.md?raw';
import environment from '../../guides/environment.md?raw';
import licenseCompliance from '../../guides/license-compliance.md?raw';
import motivation from '../../guides/motivation.md?raw';
import performance from '../../guides/performance.md?raw';
import porting from '../../guides/porting.md?raw';
import profiling from '../../guides/profiling.md?raw';
import testing from '../../guides/testing-strategy.md?raw';
import trustedPublishing from '../../guides/trusted-publishing.md?raw';
import typeAware from '../../guides/type-aware.md?raw';
import overview from '../content/overview.md?raw';
import { renderStatusMarkdown } from './status.js';

export type DocPage = {
  id: string;
  title: string;
  source: () => string;
};

export const pages: DocPage[] = [
  {
    id: 'overview',
    title: 'Overview',
    source: () => overview,
  },
  {
    id: 'status',
    title: 'Status',
    source: renderStatusMarkdown,
  },
  {
    id: 'motivation',
    title: 'Motivation',
    source: () => motivation,
  },
  {
    id: 'governance',
    title: 'Governance',
    source: () => governance,
  },
  {
    id: 'environment',
    title: 'Environment',
    source: () => environment,
  },
  {
    id: 'performance',
    title: 'Performance',
    source: () => performance,
  },
  {
    id: 'profiling',
    title: 'Profiling',
    source: () => profiling,
  },
  {
    id: 'porting',
    title: 'Porting',
    source: () => porting,
  },
  {
    id: 'testing',
    title: 'Testing',
    source: () => testing,
  },
  {
    id: 'type-aware',
    title: 'Type-Aware',
    source: () => typeAware,
  },
  {
    id: 'trusted-publishing',
    title: 'Publishing',
    source: () => trustedPublishing,
  },
  {
    id: 'license-compliance',
    title: 'Licenses',
    source: () => licenseCompliance,
  },
];
