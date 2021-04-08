using Volo.Abp.Modularity;

namespace LogBook
{
    [DependsOn(
        typeof(LogBookApplicationModule),
        typeof(LogBookDomainTestModule)
        )]
    public class LogBookApplicationTestModule : AbpModule
    {

    }
}