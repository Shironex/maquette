import { ImportSheet } from '../ImportSheet';
import { RecipeClipPicker } from '../RecipeClipPicker';
import { SaveClipsSheet } from '../SaveClipsSheet';

/** The animation library's sheets, open from any view. */
export default function AnimationDialogs() {
  return (
    <>
      <ImportSheet />
      <SaveClipsSheet />
      <RecipeClipPicker />
    </>
  );
}
