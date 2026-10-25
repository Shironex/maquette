import { useModels, type ModelsOptions } from './Models.hooks';

export type ModelsProps = ModelsOptions;

export function Models(props: ModelsProps) {
  const built = useModels(props);
  const primitives = built.map(model => <primitive key={model.entry.id} object={model.root} />);
  return <>{primitives}</>;
}
