
static PangoFont *
pango_fc_font_map_new_font (PangoFcFontMap    *fcfontmap,
			    PangoFcFontsetKey *fontset_key,
			    FcPattern         *match)
{
  PangoFcFontMapClass *class;
  PangoFcFontMapPrivate *priv = fcfontmap->priv;
  FcPattern *pattern;
  PangoFcFont *fcfont;
  PangoFcFontKey key;

  if (priv->closed)
    return NULL;

  match = uniquify_pattern (fcfontmap, match);

  pango_fc_font_key_init (&key, fcfontmap, fontset_key, match);

  fcfont = g_hash_table_lookup (priv->font_hash, &key);
  if (fcfont)
    return g_object_ref (PANGO_FONT (fcfont));

  class = PANGO_FC_FONT_MAP_GET_CLASS (fcfontmap);

  if (class->create_font)
    {
      fcfont = class->create_font (fcfontmap, &key);
    }
  else
    {
      const PangoMatrix *pango_matrix = pango_fc_fontset_key_get_matrix (fontset_key);
      FcMatrix fc_matrix, *fc_matrix_val;
      int i;

      /* Fontconfig has the Y axis pointing up, Pango, down.
       */
      fc_matrix.xx = pango_matrix->xx;
      fc_matrix.xy = - pango_matrix->xy;
      fc_matrix.yx = - pango_matrix->yx;
      fc_matrix.yy = pango_matrix->yy;

      pattern = FcPatternDuplicate (match);

      for (i = 0; FcPatternGetMatrix (pattern, FC_MATRIX, i, &fc_matrix_val) == FcResultMatch; i++)
	FcMatrixMultiply (&fc_matrix, &fc_matrix, fc_matrix_val);

      FcPatternDel (pattern, FC_MATRIX);
      FcPatternAddMatrix (pattern, FC_MATRIX, &fc_matrix);

      fcfont = class->new_font (fcfontmap, uniquify_pattern (fcfontmap, pattern));

      FcPatternDestroy (pattern);
    }

  if (!fcfont)
    return NULL;

  /* In case the backend didn't set the fontmap */
  if (!fcfont->fontmap)
    g_object_set (fcfont,
		  "fontmap", fcfontmap,
		  NULL);

  /* cache it on fontmap */
  pango_fc_font_map_add (fcfontmap, &key, fcfont);

  return (PangoFont *)fcfont;
}


static hb_font_t *
pango_fc_font_create_hb_font (PangoFont *font)
{
  PangoFcFont *fc_font = PANGO_FC_FONT (font);
  PangoFcFontKey *key;
  hb_face_t *hb_face;
  hb_font_t *hb_font;
  double x_scale_inv, y_scale_inv;
  double x_scale, y_scale;
  double pixel_size;
  double point_size;
  double slant G_GNUC_UNUSED;

  x_scale_inv = y_scale_inv = 1.0;
  pixel_size = 1.0;
  point_size = 1.0;
  slant = 0.0;

  key = _pango_fc_font_get_font_key (fc_font);
  if (key)
    {
      const FcPattern *pattern = pango_fc_font_key_get_pattern (key);
      const PangoMatrix *ctm;
      PangoMatrix font_matrix;
      PangoGravity gravity;
      FcMatrix fc_matrix, *fc_matrix_val;
      double x, y;
      int i;

      ctm = pango_fc_font_key_get_matrix (key);
      pango_matrix_get_font_scale_factors (ctm, &x_scale_inv, &y_scale_inv);

      FcMatrixInit (&fc_matrix);
      for (i = 0; FcPatternGetMatrix (pattern, FC_MATRIX, i, &fc_matrix_val) == FcResultMatch; i++)
        FcMatrixMultiply (&fc_matrix, &fc_matrix, fc_matrix_val);

      font_matrix.xx = fc_matrix.xx;
      font_matrix.yx = - fc_matrix.yx;
      font_matrix.xy = fc_matrix.xy;
      font_matrix.yy = - fc_matrix.yy;

      pango_matrix_get_font_scale_factors (&font_matrix, &x, &y);
      slant = pango_matrix_get_slant_ratio (&font_matrix);

      x_scale_inv /= x;
      y_scale_inv /= y;

      gravity = pango_fc_font_key_get_gravity (key);
      if (PANGO_GRAVITY_IS_IMPROPER (gravity))
        {
          x_scale_inv = -x_scale_inv;
          y_scale_inv = -y_scale_inv;
        }

      get_font_size (key, &pixel_size, &point_size);
    }

  x_scale = 1. / x_scale_inv;
  y_scale = 1. / y_scale_inv;

  hb_face = pango_fc_font_map_get_hb_face (PANGO_FC_FONT_MAP (fc_font->fontmap), fc_font);

  hb_font = hb_font_create (hb_face);
  hb_font_set_scale (hb_font,
                     pixel_size * PANGO_SCALE * x_scale,
                     pixel_size * PANGO_SCALE * y_scale);
  hb_font_set_ptem (hb_font, point_size);

#if HB_VERSION_ATLEAST (3, 3, 0)
  hb_font_set_synthetic_slant (hb_font, slant);
#endif

  if (key)
    {
      const FcPattern *pattern = pango_fc_font_key_get_pattern (key);
      const char *variations;
      int index;
      unsigned int n_axes;
      hb_ot_var_axis_info_t *axes;
      float *coords;
      int i;

      n_axes = hb_ot_var_get_axis_infos (hb_face, 0, NULL, NULL);
      if (n_axes == 0)
        goto done;

      axes = g_new0 (hb_ot_var_axis_info_t, n_axes);
      coords = g_new (float, n_axes);

      hb_ot_var_get_axis_infos (hb_face, 0, &n_axes, axes);
      for (i = 0; i < n_axes; i++)
        coords[axes[i].axis_index] = axes[i].default_value;

      if (FcPatternGetInteger (pattern, FC_INDEX, 0, &index) == FcResultMatch &&
          index != 0)
        {
          unsigned int instance = (index >> 16) - 1;
          hb_ot_var_named_instance_get_design_coords (hb_face, instance, &n_axes, coords);
        }

      if (FcPatternGetString (pattern, FC_FONT_VARIATIONS, 0, (FcChar8 **)&variations) == FcResultMatch)
        parse_variations (variations, axes, n_axes, coords);

      variations = pango_fc_font_key_get_variations (key);
      if (variations)
        parse_variations (variations, axes, n_axes, coords);

      hb_font_set_var_coords_design (hb_font, coords, n_axes);

      g_free (coords);
      g_free (axes);
    }

done:
  return hb_font;
}
