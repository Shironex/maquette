import { ImportSheet } from '../ImportSheet';
import { RecipeClipPicker } from '../RecipeClipPicker';
import { RecipeCompareSheet } from '../RecipeCompareSheet';
import { SaveClipsSheet } from '../SaveClipsSheet';
import { SaveGripSheet } from '../SaveGripSheet';
import { SaveRecipeSheet } from '../SaveRecipeSheet';
import { UseRecipeSheet } from '../UseRecipeSheet';

/** The animation library's sheets, open from any view. */
export default function AnimationDialogs() {
  return (
    <>
      <ImportSheet />
      <SaveClipsSheet />
      <RecipeClipPicker />
      <UseRecipeSheet />
      <SaveRecipeSheet />
      <SaveGripSheet />
      <RecipeCompareSheet />
    </>
  );
}
