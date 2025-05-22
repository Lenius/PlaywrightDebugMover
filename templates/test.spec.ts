// {{ description }}
import { test, expect } from '@playwright/test';

test.describe('{{ description }}', () => {
{% for step in steps %}
  test('{{ step }}', async ({ page }) => {
    // TODO: implement "{{ step }}"
  });
{% endfor %}
});