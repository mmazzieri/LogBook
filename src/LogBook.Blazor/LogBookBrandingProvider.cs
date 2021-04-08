using Volo.Abp.DependencyInjection;
using Volo.Abp.Ui.Branding;

namespace LogBook.Blazor
{
    [Dependency(ReplaceServices = true)]
    public class LogBookBrandingProvider : DefaultBrandingProvider
    {
        public override string AppName => "LogBook";
    }
}
