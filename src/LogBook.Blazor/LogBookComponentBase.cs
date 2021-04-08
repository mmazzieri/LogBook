using LogBook.Localization;
using Volo.Abp.AspNetCore.Components;

namespace LogBook.Blazor
{
    public abstract class LogBookComponentBase : AbpComponentBase
    {
        protected LogBookComponentBase()
        {
            LocalizationResource = typeof(LogBookResource);
        }
    }
}
