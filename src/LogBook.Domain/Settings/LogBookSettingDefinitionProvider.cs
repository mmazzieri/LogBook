using Volo.Abp.Settings;

namespace LogBook.Settings
{
    public class LogBookSettingDefinitionProvider : SettingDefinitionProvider
    {
        public override void Define(ISettingDefinitionContext context)
        {
            //Define your own settings here. Example:
            //context.Add(new SettingDefinition(LogBookSettings.MySetting1));
        }
    }
}
